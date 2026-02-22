package org.ohxorud.breom.jetbrains.meta

import org.junit.Test
import org.junit.Assert.assertTrue
import java.nio.file.Files

class BreomMetaValidatorsTest {
    @Test
    fun projectManifestRejectsImportDirective() {
        val content = """
            breom 0.1.0
            package demo
            import std.http
        """.trimIndent()

        val diagnostics = BreomMetaValidators.validateProjectManifest(content)
        assertTrue(diagnostics.any { it.message.contains("`import` is not allowed") })
    }

    @Test
    fun lockFileRequiresDependenciesArray() {
        val content = """
            {
              "dependencies": {}
            }
        """.trimIndent()

        val diagnostics = BreomMetaValidators.validateLockFile(content)
        assertTrue(diagnostics.any { it.message.contains("must be an array") })
    }

    @Test
    fun projectManifestChecksEntrypointExists() {
        val root = Files.createTempDirectory("breom_meta_test")
        val content = """
            breom 0.1.0
            package demo
            entrypoint main.brm
        """.trimIndent()

        val diagnostics = BreomMetaValidators.validateProjectManifest(content, root.toString())
        assertTrue(diagnostics.any { it.message.contains("Entrypoint file not found") })

        Files.deleteIfExists(root)
    }
}
