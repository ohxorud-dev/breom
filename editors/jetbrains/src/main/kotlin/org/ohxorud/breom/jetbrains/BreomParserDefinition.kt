package org.ohxorud.breom.jetbrains

import org.ohxorud.breom.jetbrains.lexer.BreomLexer
import org.ohxorud.breom.jetbrains.parser.BreomParser
import org.ohxorud.breom.jetbrains.psi.BreomTypes
import com.intellij.lang.ASTNode
import com.intellij.lang.ParserDefinition
import com.intellij.lang.PsiParser
import com.intellij.lexer.FlexAdapter
import com.intellij.lexer.Lexer
import com.intellij.openapi.project.Project
import com.intellij.psi.FileViewProvider
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile
import com.intellij.psi.tree.IFileElementType
import com.intellij.psi.tree.TokenSet

class BreomParserDefinition : ParserDefinition {
    companion object {
        val FILE = IFileElementType(BreomLanguage)
        val COMMENTS = TokenSet.create(BreomTypes.LINE_COMMENT, BreomTypes.BLOCK_COMMENT)
        val STRINGS = TokenSet.create(BreomTypes.STRING_LITERAL, BreomTypes.FSTRING_LITERAL, BreomTypes.MULTILINE_STRING, BreomTypes.CHAR_LITERAL)
    }

    override fun createLexer(project: Project?): Lexer = FlexAdapter(BreomLexer(null))

    override fun getCommentTokens(): TokenSet = COMMENTS

    override fun getStringLiteralElements(): TokenSet = STRINGS

    override fun createParser(project: Project?): PsiParser = BreomParser()

    override fun getFileNodeType(): IFileElementType = FILE

    override fun createFile(viewProvider: FileViewProvider): PsiFile = BreomFile(viewProvider)

    override fun createElement(node: ASTNode): PsiElement = BreomTypes.Factory.createElement(node)
}
