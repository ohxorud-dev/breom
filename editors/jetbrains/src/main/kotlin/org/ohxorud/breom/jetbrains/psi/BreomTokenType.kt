package org.ohxorud.breom.jetbrains.psi

import org.ohxorud.breom.jetbrains.BreomLanguage
import com.intellij.psi.tree.IElementType

class BreomTokenType(debugName: String) : IElementType(debugName, BreomLanguage) {
    override fun toString(): String = "BreomTokenType.$debugName"
}
