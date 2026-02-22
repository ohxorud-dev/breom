package org.ohxorud.breom.jetbrains

import com.intellij.lang.BracePair
import com.intellij.lang.PairedBraceMatcher
import com.intellij.psi.PsiFile
import com.intellij.psi.tree.IElementType
import org.ohxorud.breom.jetbrains.psi.BreomTypes

class BreomPairedBraceMatcher : PairedBraceMatcher {
    override fun getPairs(): Array<BracePair> {
        return arrayOf(
            BracePair(BreomTypes.LBRACE, BreomTypes.RBRACE, true),
            BracePair(BreomTypes.LBRACKET, BreomTypes.RBRACKET, false),
            BracePair(BreomTypes.LPAREN, BreomTypes.RPAREN, false)
        )
    }

    override fun isPairedBracesAllowedBeforeType(lbraceType: IElementType, contextType: IElementType?): Boolean {
        return true
    }

    override fun getCodeConstructStart(file: PsiFile?, openingBraceOffset: Int): Int {
        return openingBraceOffset
    }
}
