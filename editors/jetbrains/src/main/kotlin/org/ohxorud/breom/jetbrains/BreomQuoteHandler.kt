package org.ohxorud.breom.jetbrains

import com.intellij.codeInsight.editorActions.SimpleTokenSetQuoteHandler
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.editor.highlighter.HighlighterIterator
import org.ohxorud.breom.jetbrains.psi.BreomTypes

class BreomQuoteHandler : SimpleTokenSetQuoteHandler(BreomTypes.STRING_LITERAL, BreomTypes.CHAR_LITERAL) {


}
