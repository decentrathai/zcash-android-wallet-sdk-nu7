package cash.z.ecc.android.sdk.internal.db.derived

import org.junit.Test
import kotlin.test.assertEquals

class StoredAddressNormalizerTest {
    private val conversions = mutableListOf<String>()

    private fun normalizer(maxCachedStrings: Int = 8) =
        StoredAddressNormalizer(maxCachedStrings) { stored ->
            conversions += stored
            if (stored.startsWith("zu1")) stored.removePrefix("z") else stored
        }

    @Test
    fun `returns what the converter returns`() {
        val normalizer = normalizer()

        assertEquals("u1abc", normalizer.normalize("zu1abc"))
        assertEquals("u1abc", normalizer.normalize("u1abc"))
        assertEquals("t1abc", normalizer.normalize("t1abc"))
    }

    @Test
    fun `converts each stored string once`() {
        val normalizer = normalizer()

        repeat(3) { assertEquals("u1abc", normalizer.normalize("zu1abc")) }
        repeat(3) { assertEquals("u1abc", normalizer.normalize("u1abc")) }

        assertEquals(listOf("zu1abc", "u1abc"), conversions)
    }

    @Test
    fun `forgets the least recently used string beyond its limit`() {
        val normalizer = normalizer(maxCachedStrings = 2)

        normalizer.normalize("zu1a")
        normalizer.normalize("zu1b")
        normalizer.normalize("zu1a")
        normalizer.normalize("zu1c")
        normalizer.normalize("zu1a")
        normalizer.normalize("zu1b")

        assertEquals(listOf("zu1a", "zu1b", "zu1c", "zu1b"), conversions)
    }
}
