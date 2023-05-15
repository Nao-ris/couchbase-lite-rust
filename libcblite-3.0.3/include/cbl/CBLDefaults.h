//
//  CBLDefaults.h
//  CouchbaseLite
//
//  Copyright (c) 2023-present Couchbase, Inc All rights reserved.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//

// THIS IS AN AUTOGENERATED FILE, MANUAL CHANGES SHOULD BE EXPECTED TO
// BE OVERWRITTEN


#pragma once
#include "CBL_Compat.h"
#include "CBLReplicator.h"

CBL_CAPI_BEGIN

/** \defgroup constants   Constants

     @{

    Constants for default configuration values. */

/** \name CBLLogFileConfiguration
    @{
*/

/** [false] Plaintext is not used, and instead binary encoding is used in log files */
CBL_PUBLIC extern const bool kCBLDefaultLogFileUsePlainText;

/** [524288] 512 KiB for the size of a log file */
CBL_PUBLIC extern const size_t kCBLDefaultLogFileMaxSize;

/** [1] 1 rotated file present (2 total, including the currently active log file) */
CBL_PUBLIC extern const uint32_t kCBLDefaultLogFileMaxRotateCount;


/** @} */

/** \name CBLFullTextIndexConfiguration
    @{
*/

/** [false] Accents and ligatures are not ignored when indexing via full text search */
CBL_PUBLIC extern const bool kCBLDefaultFullTextIndexIgnoreAccents;


/** @} */

/** \name CBLReplicatorConfiguration
    @{
*/

/** [kCBLReplicatorTypePushAndPull] Perform bidirectional replication */
CBL_PUBLIC extern const CBLReplicatorType kCBLDefaultReplicatorType;

/** [false] One-shot replication is used, and will stop once all initial changes are processed */
CBL_PUBLIC extern const bool kCBLDefaultReplicatorContinuous;

/** [300] A heartbeat messages is sent every 300 seconds to keep the connection alive */
CBL_PUBLIC extern const unsigned kCBLDefaultReplicatorHeartbeat;

/** [10] When replicator is not continuous, after 10 failed attempts give up on the replication */
CBL_PUBLIC extern const unsigned kCBLDefaultReplicatorMaxAttemptsSingleShot;

/** [UINT_MAX] When replicator is continuous, never give up unless explicitly stopped */
CBL_PUBLIC extern const unsigned kCBLDefaultReplicatorMaxAttemptsContinuous;

/** [300] Max wait time between retry attempts in seconds */
CBL_PUBLIC extern const unsigned kCBLDefaultReplicatorMaxAttemptWaitTime;

/** [false] Purge documents when a user loses access */
CBL_PUBLIC extern const bool kCBLDefaultReplicatorDisableAutoPurge;

/** [false] Whether or not a replicator only accepts cookies for the sender's parent domains */
CBL_PUBLIC extern const bool kCBLDefaultReplicatorAcceptParentCookies;

/** @} */

/** @} */

CBL_CAPI_END
