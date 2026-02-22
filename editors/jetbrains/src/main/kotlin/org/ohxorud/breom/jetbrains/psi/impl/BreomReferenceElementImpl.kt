package org.ohxorud.breom.jetbrains.psi.impl

import com.intellij.extapi.psi.ASTWrapperPsiElement
import com.intellij.lang.ASTNode
import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiReference
import org.ohxorud.breom.jetbrains.psi.BreomElementFactory
import org.ohxorud.breom.jetbrains.psi.BreomReference

abstract class BreomReferenceElementImpl(node: ASTNode) : ASTWrapperPsiElement(node) {
    override fun getReference(): PsiReference? {
        return BreomReference(this, TextRange(0, textLength))
    }

    override fun getName(): String? {
        return text
    }

    open fun setName(name: String): PsiElement {
        val newElement = BreomElementFactory.createIdentifier(project, name)
        if (newElement != null) {
            node.treeParent.replaceChild(node, newElement.node)
        }
        return this
    }

    open fun getNameIdentifier(): PsiElement? {
        return firstChild
    }
}
