package org.ohxorud.breom.jetbrains.grammar

import org.junit.Assert.assertTrue
import org.junit.Test
import java.nio.file.Files
import java.nio.file.Path

class GrammarParitySmokeTest {
    @Test
    fun bnf_contains_core_constructs_from_pest() {
        val repoRoot = Path.of("..", "..").normalize()
        val pestPath = repoRoot.resolve("src/breom.pest")
        val bnfPath = repoRoot.resolve("editors/jetbrains/src/main/grammars/Breom.bnf")

        val pest = Files.readString(pestPath)
        val bnf = Files.readString(bnfPath)

        assertTrue("pest must define enum", pest.contains("enum_decl"))
        assertTrue("bnf must define enum", bnf.contains("enumDecl ::= "))

        assertTrue("pest must define throw", pest.contains("throw_stmt"))
        assertTrue("bnf must define throw", bnf.contains("throwStmt ::= "))

        assertTrue("pest must define catch_block", pest.contains("catch_block"))
        assertTrue("bnf must define catchBlock", bnf.contains("catchBlock ::= "))

        assertTrue("pest must define instead_fallback", pest.contains("instead_fallback"))
        assertTrue("bnf must define insteadFallback", bnf.contains("insteadFallback ::= "))

        assertTrue("pest must define wait_timeout", pest.contains("wait_timeout"))
        assertTrue("bnf must define waitTimeout", bnf.contains("waitTimeout ::= "))
    }
}
