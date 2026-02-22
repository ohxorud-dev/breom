package org.ohxorud.breom.jetbrains

import org.ohxorud.breom.jetbrains.psi.BreomTypes
import com.intellij.lexer.Lexer
import com.intellij.lexer.FlexAdapter
import org.ohxorud.breom.jetbrains.lexer.BreomLexer
import com.intellij.openapi.editor.DefaultLanguageHighlighterColors
import com.intellij.openapi.editor.HighlighterColors
import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.editor.colors.TextAttributesKey.createTextAttributesKey
import com.intellij.openapi.fileTypes.SyntaxHighlighter
import com.intellij.openapi.fileTypes.SyntaxHighlighterBase
import com.intellij.openapi.fileTypes.SyntaxHighlighterFactory
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.tree.IElementType

class BreomSyntaxHighlighterFactory : SyntaxHighlighterFactory() {
    override fun getSyntaxHighlighter(project: Project?, virtualFile: VirtualFile?): SyntaxHighlighter = BreomSyntaxHighlighter()
}

class BreomSyntaxHighlighter : SyntaxHighlighterBase() {
    companion object {
        val KEYWORD = createTextAttributesKey("BREOM_KEYWORD", DefaultLanguageHighlighterColors.KEYWORD)
        val STRING = createTextAttributesKey("BREOM_STRING", DefaultLanguageHighlighterColors.STRING)
        val NUMBER = createTextAttributesKey("BREOM_NUMBER", DefaultLanguageHighlighterColors.NUMBER)
        val COMMENT = createTextAttributesKey("BREOM_COMMENT", DefaultLanguageHighlighterColors.LINE_COMMENT)
        val BLOCK_COMMENT = createTextAttributesKey("BREOM_BLOCK_COMMENT", DefaultLanguageHighlighterColors.BLOCK_COMMENT)
        val OPERATOR = createTextAttributesKey("BREOM_OPERATOR", DefaultLanguageHighlighterColors.OPERATION_SIGN)
        val PAREN = createTextAttributesKey("BREOM_PAREN", DefaultLanguageHighlighterColors.PARENTHESES)
        val BRACE = createTextAttributesKey("BREOM_BRACE", DefaultLanguageHighlighterColors.BRACES)
        val BRACKET = createTextAttributesKey("BREOM_BRACKET", DefaultLanguageHighlighterColors.BRACKETS)
        val IDENTIFIER = createTextAttributesKey("BREOM_IDENTIFIER", DefaultLanguageHighlighterColors.IDENTIFIER)
        val TYPE = createTextAttributesKey("BREOM_TYPE", DefaultLanguageHighlighterColors.CLASS_NAME)
        val ATTRIBUTE = createTextAttributesKey("BREOM_ATTRIBUTE", DefaultLanguageHighlighterColors.METADATA)
        val BAD_CHAR = createTextAttributesKey("BREOM_BAD_CHARACTER", HighlighterColors.BAD_CHARACTER)

        private val KEYWORD_KEYS = arrayOf(KEYWORD)
        private val STRING_KEYS = arrayOf(STRING)
        private val NUMBER_KEYS = arrayOf(NUMBER)
        private val COMMENT_KEYS = arrayOf(COMMENT)
        private val BLOCK_COMMENT_KEYS = arrayOf(BLOCK_COMMENT)
        private val OPERATOR_KEYS = arrayOf(OPERATOR)
        private val PAREN_KEYS = arrayOf(PAREN)
        private val BRACE_KEYS = arrayOf(BRACE)
        private val BRACKET_KEYS = arrayOf(BRACKET)
        private val IDENTIFIER_KEYS = arrayOf(IDENTIFIER)
        private val TYPE_KEYS = arrayOf(TYPE)
        private val ATTRIBUTE_KEYS = arrayOf(ATTRIBUTE)
        private val BAD_CHAR_KEYS = arrayOf(BAD_CHAR)
        private val EMPTY_KEYS = emptyArray<TextAttributesKey>()
    }

    override fun getHighlightingLexer(): Lexer = FlexAdapter(BreomLexer(null))

    override fun getTokenHighlights(tokenType: IElementType?): Array<TextAttributesKey> {
        return when (tokenType) {

            BreomTypes.PACKAGE_KEYWORD, BreomTypes.BREOM_KEYWORD, BreomTypes.ENTRYPOINT_KEYWORD,
            BreomTypes.DEP_KEYWORD, BreomTypes.IMPORT_KEYWORD,
            BreomTypes.AS_KEYWORD, BreomTypes.PUB_KEYWORD, BreomTypes.MUT_KEYWORD,
            BreomTypes.DEFINE_KEYWORD, BreomTypes.FN_KEYWORD, BreomTypes.RETURN_KEYWORD,
            BreomTypes.DEFER_KEYWORD, BreomTypes.FOR_KEYWORD, BreomTypes.RANGE_KEYWORD,
            BreomTypes.IF_KEYWORD, BreomTypes.ELSE_KEYWORD, BreomTypes.STRUCT_KEYWORD,
            BreomTypes.INTERFACE_KEYWORD, BreomTypes.ENUM_KEYWORD, BreomTypes.ATTRIBUTE_KEYWORD,
            BreomTypes.MATCH_KEYWORD, BreomTypes.SPAWN_KEYWORD, BreomTypes.WAIT_KEYWORD,
            BreomTypes.DEFAULT_KEYWORD, BreomTypes.TIMEOUT_KEYWORD, BreomTypes.POINT_KEYWORD,
            BreomTypes.TEST_KEYWORD, BreomTypes.NEW_KEYWORD, BreomTypes.OP_KEYWORD,
            BreomTypes.THROW_KEYWORD, BreomTypes.THROWS_KEYWORD, BreomTypes.CATCH_KEYWORD,
            BreomTypes.INSTEAD_KEYWORD, BreomTypes.VOID_KEYWORD, BreomTypes.TRUE_KEYWORD,
            BreomTypes.FALSE_KEYWORD, BreomTypes.SHL_KEYWORD, BreomTypes.SHR_KEYWORD,
            BreomTypes.BREAK_KEYWORD, BreomTypes.CONTINUE_KEYWORD, BreomTypes.IN_KEYWORD,
            BreomTypes.TUPLE_KEYWORD, BreomTypes.CHAN_KEYWORD, BreomTypes.SELF_KEYWORD -> KEYWORD_KEYS


            BreomTypes.STRING_LITERAL, BreomTypes.FSTRING_LITERAL,
            BreomTypes.MULTILINE_STRING, BreomTypes.CHAR_LITERAL -> STRING_KEYS


            BreomTypes.INTEGER_LITERAL, BreomTypes.FLOAT_LITERAL -> NUMBER_KEYS


            BreomTypes.LINE_COMMENT -> COMMENT_KEYS
            BreomTypes.BLOCK_COMMENT -> BLOCK_COMMENT_KEYS


            BreomTypes.PLUS, BreomTypes.MINUS, BreomTypes.STAR, BreomTypes.SLASH,
            BreomTypes.PERCENT, BreomTypes.EQ, BreomTypes.EQEQ, BreomTypes.NEQ,
            BreomTypes.LT, BreomTypes.GT, BreomTypes.LTEQ, BreomTypes.GTEQ,
            BreomTypes.ANDAND, BreomTypes.OROR, BreomTypes.NOT, BreomTypes.AND,
            BreomTypes.OR, BreomTypes.XOR, BreomTypes.TILDE, BreomTypes.PLUSEQ,
            BreomTypes.MINUSEQ, BreomTypes.STAREQ, BreomTypes.SLASHEQ, BreomTypes.PERCENTEQ,
            BreomTypes.ANDEQ, BreomTypes.OREQ, BreomTypes.XOREQ, BreomTypes.ARROW,
            BreomTypes.FATARROW, BreomTypes.COLONCOLON, BreomTypes.COLONEQ,
            BreomTypes.QUESTION, BreomTypes.LSHIFT -> OPERATOR_KEYS


            BreomTypes.LPAREN, BreomTypes.RPAREN -> PAREN_KEYS
            BreomTypes.LBRACE, BreomTypes.RBRACE -> BRACE_KEYS
            BreomTypes.LBRACKET, BreomTypes.RBRACKET -> BRACKET_KEYS


            BreomTypes.AT -> ATTRIBUTE_KEYS


            BreomTypes.TYPE_IDENTIFIER -> TYPE_KEYS
            BreomTypes.IDENTIFIER -> IDENTIFIER_KEYS

            else -> EMPTY_KEYS
        }
    }
}
