package org.ohxorud.breom.jetbrains.psi.impl

import org.ohxorud.breom.jetbrains.psi.BreomTypes
import com.intellij.extapi.psi.ASTWrapperPsiElement
import com.intellij.lang.ASTNode
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiNameIdentifierOwner
import org.ohxorud.breom.jetbrains.psi.BreomElementFactory
import org.ohxorud.breom.jetbrains.psi.BreomNamedElement

abstract class BreomNamedElementImpl(node: ASTNode) : ASTWrapperPsiElement(node), BreomNamedElement {
    override fun getName(): String? {
        return nameIdentifier?.text
    }

    override fun setName(name: String): PsiElement {
        val keyNode = nameIdentifier ?: return this
        val newKeyNode = BreomElementFactory.createIdentifier(project, name)
        if (newKeyNode != null) {
            node.replaceChild(keyNode.node, newKeyNode.node)
        }
        return this
    }

    override fun getNameIdentifier(): PsiElement? {
        return node.findChildByType(BreomTypes.IDENTIFIER)?.psi
            ?: node.findChildByType(BreomTypes.TYPE_IDENTIFIER)?.psi
    }

    override fun getTextOffset(): Int {
        return nameIdentifier?.textOffset ?: super.getTextOffset()
    }
}
