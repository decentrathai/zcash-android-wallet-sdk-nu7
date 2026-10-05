package cash.z.ecc.android.sdk.internal.jni

/**
 * ZIP 316 Revision 0 encodings of Unified Addresses and Unified Viewing Keys.
 *
 * The native library returns Revision 0 strings (`u1…`, `uview1…`, `uivk1…`), as SDK 3.0.0 did, but the wallet
 * database stores the addresses this build creates in their Revision 2 form (`zu…`, `tu…`). Strings read from the
 * database go through [toRevision0] so that an address is always returned as the same string.
 */
object RustUnifiedEncoding {
    /**
     * Returns the Revision 0 form of a Unified Address, Unified Full Viewing Key or Unified Incoming Viewing Key
     * string. Any other string (a transparent, Sapling or TEX address, a string that is already Revision 0, or one
     * that has no Revision 0 form) is returned unchanged.
     *
     * The native library must be loaded, see [RustBackend.loadLibrary].
     */
    @JvmStatic
    external fun toRevision0(encoded: String): String
}
