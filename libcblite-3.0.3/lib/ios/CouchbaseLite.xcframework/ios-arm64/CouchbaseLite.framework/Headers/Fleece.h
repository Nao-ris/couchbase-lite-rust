//
// Fleece.h
//
// Copyright 2016-Present Couchbase, Inc.
//
// Use of this software is governed by the Business Source License included
// in the file licenses/BSL-Couchbase.txt.  As of the Change Date specified
// in that file, in accordance with the Business Source License, use of this
// software will be governed by the Apache License, Version 2.0, included in
// the file licenses/APL2.txt.
//

#pragma once
#ifndef _FLEECE_H
#define _FLEECE_H

// This "umbrella header" includes the commonly-used parts of the Fleece C API.

#include <CouchbaseLite/FLBase.h>
#include <CouchbaseLite/FLCollections.h>
#include <CouchbaseLite/FLDeepIterator.h>
#include <CouchbaseLite/FLDoc.h>
#include <CouchbaseLite/FLEncoder.h>
#include <CouchbaseLite/FLJSON.h>
#include <CouchbaseLite/FLKeyPath.h>
#include <CouchbaseLite/FLMutable.h>
#include <CouchbaseLite/FLValue.h>

// #include <CouchbaseLite/FLExpert.h>  -- advanced & rarely-used functionality

#ifdef __OBJC__
    // When compiling as Objective-C, include CoreFoundation / Objective-C utilities:
#include <CouchbaseLite/Fleece+CoreFoundation.h>
#endif

#endif // _FLEECE_H
