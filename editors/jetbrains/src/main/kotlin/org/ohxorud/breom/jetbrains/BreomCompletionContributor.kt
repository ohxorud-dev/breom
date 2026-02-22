package org.ohxorud.breom.jetbrains

import com.intellij.codeInsight.completion.*
import com.intellij.codeInsight.lookup.LookupElementBuilder
import com.intellij.patterns.PlatformPatterns
import com.intellij.util.ProcessingContext
import org.ohxorud.breom.jetbrains.psi.BreomModulePath
import org.ohxorud.breom.jetbrains.psi.BreomTypes
import org.ohxorud.breom.jetbrains.psi.BreomUtil

class BreomCompletionContributor : CompletionContributor() {
    private val KEYWORDS = listOf(
        "as", "attribute", "break", "breom", "catch", "chan", "continue", "default", "dep",
        "defer", "define", "else", "entrypoint", "enum", "false", "fn", "for", "if", "import", "in", "instead",
        "interface", "match", "mut", "new", "op", "package", "point", "pub", "range", "return",
        "self", "shl", "shr", "spawn", "struct", "test", "throw", "throws", "timeout", "to",
        "true", "Tuple", "void", "wait"
    )

    init {
        extend(
            CompletionType.BASIC,
            PlatformPatterns.psiElement(BreomTypes.IDENTIFIER).withLanguage(BreomLanguage),
            object : CompletionProvider<CompletionParameters>() {
                override fun addCompletions(
                    parameters: CompletionParameters,
                    context: ProcessingContext,
                    resultSet: CompletionResultSet
                ) {
                    for (keyword in KEYWORDS) {
                        resultSet.addElement(LookupElementBuilder.create(keyword).withBoldness(true))
                    }
                }
            }
        )

        extend(
            CompletionType.BASIC,
            PlatformPatterns.psiElement().withParent(BreomModulePath::class.java).withLanguage(BreomLanguage),
            object : CompletionProvider<CompletionParameters>() {
                override fun addCompletions(
                    parameters: CompletionParameters,
                    context: ProcessingContext,
                    resultSet: CompletionResultSet,
                ) {
                    for (module in BreomUtil.findAvailableModules(parameters.position.project)) {
                        resultSet.addElement(
                            LookupElementBuilder.create(module)
                                .withTypeText("module")
                        )
                    }
                }
            }
        )
    }
}
