// Couchbase Lite query API
//
// Copyright (c) 2020 Couchbase, Inc All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use crate::{
    Array, CblRef, CouchbaseLiteError, Database, Dict, Error, MutableDict, Result, Value, failure,
    release, retain,
    slice::from_str,
    c_api::{
        CBLDatabase_CreateQuery, CBLError, CBLQuery, CBLQueryLanguage, CBLQuery_ColumnCount,
        CBLQuery_ColumnName, CBLQuery_Execute, CBLQuery_Explain, CBLQuery_Parameters,
        CBLQuery_SetParameters, CBLResultSet, CBLResultSet_GetQuery, CBLResultSet_Next,
        CBLResultSet_ResultArray, CBLResultSet_ResultDict, CBLResultSet_ValueAtIndex,
        CBLResultSet_ValueForKey, CBLListenerToken, CBLQuery_AddChangeListener,
        CBLQuery_CopyCurrentResults,
    },
    Listener,
};

use std::{os::raw::c_uint};
use ListenerToken;

/** Query languages. */
pub enum QueryLanguage {
    JSON, // JSON query schema: github.com/couchbase/couchbase-lite-core/wiki/JSON-Query-Schema
    N1QL, // N1QL syntax: docs.couchbase.com/server/6.0/n1ql/n1ql-language-reference/index.html
}

type ChangeListener = Box<dyn Fn(&Query, &ListenerToken)>;

#[no_mangle]
unsafe extern "C" fn c_query_change_listener(
    context: *mut ::std::os::raw::c_void,
    query: *mut CBLQuery,
    token: *mut CBLListenerToken,
) {
    let callback = context as *const ChangeListener;
    let query = Query::wrap(query.cast::<CBLQuery>());
    let token = ListenerToken::new(token);

    (*callback)(&query, &token);
}

/** A compiled database query. */
pub struct Query {
    cbl_ref: *mut CBLQuery,
}

impl CblRef for Query {
    type Output = *mut CBLQuery;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Query {
    /** Creates a new query by compiling the input string.
    This is fast, but not instantaneous. If you need to run the same query many times, keep the
    `Query` around instead of compiling it each time. If you need to run related queries
    with only some values different, create one query with placeholder parameter(s), and substitute
    the desired value(s) with `set_parameters` before each time you run the query. */
    pub fn new(db: &Database, language: QueryLanguage, str: &str) -> Result<Self> {
        unsafe {
            let mut pos: i32 = 0;
            let mut err = CBLError::default();
            let q = CBLDatabase_CreateQuery(
                db.get_ref(),
                language as CBLQueryLanguage,
                from_str(str).get_ref(),
                &mut pos,
                &mut err,
            );
            if q.is_null() {
                // TODO: Return the error pos somehow
                return failure(err);
            }

            Ok(Self { cbl_ref: q })
        }
    }

    pub(crate) fn wrap(cbl_ref: *mut CBLQuery) -> Self {
        Self {
            cbl_ref: unsafe { retain(cbl_ref) },
        }
    }

    /** Assigns values to the query's parameters.
    These values will be substited for those parameters whenever the query is executed,
    until they are next assigned.

    Parameters are specified in the query source as
    e.g. `$PARAM` (N1QL) or `["$PARAM"]` (JSON). In this example, the `parameters` dictionary
    to this call should have a key `PARAM` that maps to the value of the parameter. */
    pub fn set_parameters(&self, parameters: &MutableDict) {
        unsafe {
            CBLQuery_SetParameters(self.get_ref(), parameters.get_ref());
        }
    }

    /** Returns the query's current parameter bindings, if any. */
    pub fn parameters(&self) -> Dict {
        unsafe {
            Dict {
                cbl_ref: CBLQuery_Parameters(self.get_ref()),
            }
        }
    }

    /** Returns information about the query, including the translated SQLite form, and the search
    strategy. You can use this to help optimize the query: the word `SCAN` in the strategy
    indicates a linear scan of the entire database, which should be avoided by adding an index.
    The strategy will also show which index(es), if any, are used. */
    pub fn explain(&self) -> Result<String> {
        unsafe {
            CBLQuery_Explain(self.get_ref())
                .to_string()
                .ok_or(Error::cbl_error(CouchbaseLiteError::InvalidQuery))
        }
    }

    /** Runs the query, returning the results as a `ResultSet` object, which is an iterator
    of `Row` objects, each of which has column values. */
    pub fn execute(&self) -> Result<ResultSet> {
        unsafe {
            let mut err = CBLError::default();
            let r = CBLQuery_Execute(self.get_ref(), &mut err);
            if r.is_null() {
                return failure(err);
            }
            Ok(ResultSet { cbl_ref: r })
        }
    }

    /** Returns the number of columns in each result.
    This comes directly from the number of "SELECT..." values in the query string. */
    pub fn column_count(&self) -> usize {
        unsafe { CBLQuery_ColumnCount(self.get_ref()) as usize }
    }

    /** Returns the name of a column in the result.
    The column name is based on its expression in the `SELECT...` or `WHAT:` section of the
    query. A column that returns a property or property path will be named after that property.
    A column that returns an expression will have an automatically-generated name like `$1`.
    To give a column a custom name, use the `AS` syntax in the query.
    Every column is guaranteed to have a unique name. */
    pub fn column_name(&self, col: usize) -> Option<&str> {
        unsafe { CBLQuery_ColumnName(self.get_ref(), col as u32).as_str() }
    }

    /** Returns the column names as a Vec. */
    pub fn column_names(&self) -> Vec<&str> {
        (0..self.column_count())
            .map(|i| self.column_name(i).unwrap())
            .collect()
    }

    /** Registers a change listener callback with a query, turning it into a "live query" until
    the listener is removed (via \ref CBLListener_Remove).

    When the first change listener is added, the query will run (in the background) and notify
    the listener(s) of the results when ready. After that, it will run in the background after
    the database changes, and only notify the listeners when the result set changes.

    # Lifetime

    The listener is deleted at the end of life of the `Listener` object.
    You must keep the `Listener` object as long as you need it.
    */
    #[must_use]
    pub fn add_listener(&mut self, listener: ChangeListener) -> Listener<ChangeListener> {
        unsafe {
            let listener = Box::new(listener);
            let ptr = Box::into_raw(listener);

            Listener::new(
                ListenerToken::new(CBLQuery_AddChangeListener(
                    self.get_ref(),
                    Some(c_query_change_listener),
                    ptr.cast(),
                )),
                Box::from_raw(ptr),
            )
        }
    }

    pub fn copy_current_results(&self, listener: &ListenerToken) -> Result<ResultSet> {
        let mut error = CBLError::default();
        let result =
            unsafe { CBLQuery_CopyCurrentResults(self.get_ref(), listener.get_ref(), &mut error) };
        if result.is_null() {
            return failure(error);
        }
        Ok(ResultSet { cbl_ref: result })
    }
}

impl Drop for Query {
    fn drop(&mut self) {
        unsafe {
            release(self.get_ref());
        }
    }
}

impl Clone for Query {
    fn clone(&self) -> Self {
        unsafe {
            Self {
                cbl_ref: retain(self.get_ref()),
            }
        }
    }
}

//////// RESULT SET:

/** An iterator over the rows resulting from running a query. */
pub struct ResultSet {
    cbl_ref: *mut CBLResultSet,
}

impl CblRef for ResultSet {
    type Output = *mut CBLResultSet;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Iterator for ResultSet {
    type Item = Row;

    fn next(&mut self) -> Option<Row> {
        unsafe {
            if !CBLResultSet_Next(self.get_ref()) {
                return None;
            }
            Some(Row {
                cbl_ref: self.get_ref(),
            })
        }
    }
}

impl Drop for ResultSet {
    fn drop(&mut self) {
        unsafe {
            release(self.get_ref());
        }
    }
}

//////// ROW:

/** A single result row from a Query. */
pub struct Row {
    cbl_ref: *mut CBLResultSet,
}

impl Row {
    /** Returns the value of a column, given its (zero-based) index. */
    pub fn get(&self, index: isize) -> Value {
        unsafe {
            Value {
                cbl_ref: CBLResultSet_ValueAtIndex(self.cbl_ref, index as c_uint),
            }
        }
    }

    /** Returns the value of a column, given its name. */
    pub fn get_key(&self, key: &str) -> Value {
        unsafe {
            Value {
                cbl_ref: CBLResultSet_ValueForKey(self.cbl_ref, from_str(key).get_ref()),
            }
        }
    }

    /** Returns the number of columns. (This is the same as `Query`::column_count.) */
    pub fn column_count(&self) -> isize {
        unsafe {
            let query = CBLResultSet_GetQuery(self.cbl_ref);
            CBLQuery_ColumnCount(query) as isize
        }
    }

    /** Returns the name of a column. */
    pub fn column_name(&self, col: isize) -> Option<&str> {
        unsafe {
            let query = CBLResultSet_GetQuery(self.cbl_ref);
            CBLQuery_ColumnName(query, col as c_uint).as_str()
        }
    }

    /** Returns all of the columns as a Fleece array. */
    pub fn as_array(&self) -> Array {
        unsafe {
            Array {
                cbl_ref: CBLResultSet_ResultArray(self.cbl_ref),
            }
        }
    }

    /** Returns all of the columns as a Fleece dictionary. */
    pub fn as_dict(&self) -> Dict {
        unsafe {
            Dict {
                cbl_ref: CBLResultSet_ResultDict(self.cbl_ref),
            }
        }
    }
}
