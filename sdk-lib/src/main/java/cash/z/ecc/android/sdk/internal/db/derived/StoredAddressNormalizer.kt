package cash.z.ecc.android.sdk.internal.db.derived

import cash.z.ecc.android.sdk.internal.jni.RustUnifiedEncoding

/**
 * Converts address strings read from the wallet database to the form the SDK returns everywhere else.
 *
 * The database holds Unified Addresses in two encodings: rows written by SDK 3.0.0 are ZIP 316 Revision 0 (`u1…`),
 * rows written by this build are Revision 2 (`zu…`, `tu…`). Apps compare address strings, so every row is returned in
 * Revision 0; the stored rows themselves are never rewritten. Conversions are cached per string because the same few
 * addresses repeat across many transaction outputs.
 */
internal class StoredAddressNormalizer(
    private val maxCachedStrings: Int = MAX_CACHED_STRINGS,
    private val toRevision0: (String) -> String = RustUnifiedEncoding::toRevision0,
) {
    private val cache =
        object : LinkedHashMap<String, String>(INITIAL_CAPACITY, LOAD_FACTOR, true) {
            override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, String>) = size > maxCachedStrings
        }

    fun normalize(stored: String): String =
        synchronized(cache) {
            cache.getOrPut(stored) { toRevision0(stored) }
        }

    private companion object {
        const val MAX_CACHED_STRINGS = 512
        const val INITIAL_CAPACITY = 16
        const val LOAD_FACTOR = 0.75f
    }
}
