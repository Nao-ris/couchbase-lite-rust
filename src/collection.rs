use crate::{
    CblRef, Listener, ListenerToken, release, retain, check_error,
    c_api::{
        CBLCollection, CBLCollectionChange, CBLScope, CBLCollection_AddChangeListener,
        CBLCollection_Scope, CBLCollection_Name, CBLCollection_Count, CBLScope_Name,
        CBLScope_CollectionNames, CBLScope_Collection, CBLError,
    },
    error::Result,
    MutableArray,
    slice::from_str,
};

pub static DEFAULT_NAME: &str = "_default";

#[derive(Debug, PartialEq, Eq)]
pub struct Collection {
    cbl_ref: *mut CBLCollection,
}

impl Collection {
    pub(crate) fn retain(cbl_ref: *mut CBLCollection) -> Self {
        Self {
            cbl_ref: unsafe { retain(cbl_ref) },
        }
    }

    /** Returns the scope of the collection. */
    pub fn scope(&self) -> Scope {
        unsafe { Scope::retain(CBLCollection_Scope(self.get_ref())) }
    }

    /** Returns the collection name. */
    pub fn name(&self) -> String {
        unsafe {
            CBLCollection_Name(self.get_ref())
                .to_string()
                .unwrap_or_default()
        }
    }

    /** Returns the number of documents in the collection. */
    pub fn count(&self) -> u64 {
        unsafe { CBLCollection_Count(self.get_ref()) }
    }

    /** Registers a collection change listener callback. It will be called after one or more documents are changed on disk. */
    pub fn add_listener(
        &mut self,
        listener: CollectionChangeListener,
    ) -> Listener<CollectionChangeListener> {
        unsafe {
            let listener = Box::new(listener);
            let ptr = Box::into_raw(listener);

            Listener::new(
                ListenerToken {
                    cbl_ref: CBLCollection_AddChangeListener(
                        self.get_ref(),
                        Some(c_collection_change_listener),
                        ptr.cast(),
                    ),
                },
                Box::from_raw(ptr),
            )
        }
    }
}

impl CblRef for Collection {
    type Output = *mut CBLCollection;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Drop for Collection {
    fn drop(&mut self) {
        unsafe { release(self.get_ref()) }
    }
}

impl Clone for Collection {
    fn clone(&self) -> Self {
        Self::retain(self.get_ref())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Scope {
    cbl_ref: *mut CBLScope,
}

/** A collection change listener callback, invoked after one or more documents are changed on disk. */
type CollectionChangeListener = Box<dyn Fn(Collection, Vec<String>)>;

#[no_mangle]
unsafe extern "C" fn c_collection_change_listener(
    context: *mut ::std::os::raw::c_void,
    change: *const CBLCollectionChange,
) {
    let callback = context as *const CollectionChangeListener;
    if let Some(change) = change.as_ref() {
        let collection = Collection::retain(change.collection as *mut CBLCollection);
        let doc_ids = std::slice::from_raw_parts(change.docIDs, change.numDocs as usize)
            .iter()
            .filter_map(|doc_id| doc_id.to_string())
            .collect();

        (*callback)(collection, doc_ids);
    }
}

impl Scope {
    pub(crate) fn retain(cbl_ref: *mut CBLScope) -> Self {
        Self {
            cbl_ref: unsafe { retain(cbl_ref) },
        }
    }

    /** Returns the name of the scope. */
    pub fn name(&self) -> String {
        unsafe {
            CBLScope_Name(self.get_ref())
                .to_string()
                .unwrap_or_default()
        }
    }

    /** Returns the names of all collections in the scope. */
    pub fn collection_names(&self) -> Result<Vec<String>> {
        let mut error = CBLError::default();
        let array = unsafe { CBLScope_CollectionNames(self.get_ref(), &mut error) };

        check_error(&error).map(|()| unsafe {
            MutableArray::adopt(array)
                .iter()
                .map(|v| v.as_string().unwrap_or("").to_string())
                .collect()
        })
    }

    /** Returns an existing collection in the scope with the given name.*/
    pub fn collection(&self, collection_name: String) -> Result<Option<Collection>> {
        let collection_name = from_str(&collection_name);
        let mut error = CBLError::default();
        let collection =
            unsafe { CBLScope_Collection(self.get_ref(), collection_name.get_ref(), &mut error) };

        check_error(&error).map(|()| {
            if collection.is_null() {
                None
            } else {
                Some(Collection::retain(collection))
            }
        })
    }
}

impl CblRef for Scope {
    type Output = *mut CBLScope;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        unsafe { release(self.get_ref()) }
    }
}

impl Clone for Scope {
    fn clone(&self) -> Self {
        Self::retain(self.get_ref())
    }
}
