package org.ohxorud.breom.jetbrains

import org.ohxorud.breom.jetbrains.lexer.BreomLexer
import org.ohxorud.breom.jetbrains.psi.BreomTypes
import com.intellij.lang.cacheBuilder.DefaultWordsScanner
import com.intellij.lang.cacheBuilder.WordsScanner
import com.intellij.lang.findUsages.FindUsagesProvider
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiNamedElement
import com.intellij.psi.tree.TokenSet
import org.ohxorud.breom.jetbrains.psi.BreomNamedElement
import org.ohxorud.breom.jetbrains.psi.BreomDefineDecl
import org.ohxorud.breom.jetbrains.psi.BreomEnumDecl
import org.ohxorud.breom.jetbrains.psi.BreomFunctionDecl
import org.ohxorud.breom.jetbrains.psi.BreomMethodDecl
import org.ohxorud.breom.jetbrains.psi.BreomStructDecl
import org.ohxorud.breom.jetbrains.psi.BreomVarDecl

import com.intellij.lexer.FlexAdapter

class BreomFindUsagesProvider : FindUsagesProvider {
    override fun getWordsScanner(): WordsScanner? {
        return DefaultWordsScanner(
            FlexAdapter(BreomLexer(null)),
            TokenSet.create(BreomTypes.IDENTIFIER),
            BreomParserDefinition.COMMENTS,
            BreomParserDefinition.STRINGS
        )
    }

    override fun canFindUsagesFor(psiElement: PsiElement): Boolean {
        return psiElement is BreomNamedElement
    }

    override fun getHelpId(psiElement: PsiElement): String? {
        return null
    }

    override fun getType(element: PsiElement): String {
        return when (element) {
            is BreomFunctionDecl -> "function"
            is BreomMethodDecl -> "method"
            is BreomStructDecl -> "struct"
            is BreomEnumDecl -> "enum"
            is BreomDefineDecl -> "define"
            is BreomVarDecl -> "variable"
            is BreomNamedElement -> "symbol"
            else -> ""
        }
    }

    override fun getDescriptiveName(element: PsiElement): String {
        return if (element is PsiNamedElement) element.name ?: "" else ""
    }

    override fun getNodeText(element: PsiElement, useFullName: Boolean): String {
        return if (element is PsiNamedElement) element.name ?: "" else ""
    }
}
