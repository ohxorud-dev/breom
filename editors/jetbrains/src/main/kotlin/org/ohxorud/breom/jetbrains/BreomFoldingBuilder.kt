package org.ohxorud.breom.jetbrains

import com.intellij.lang.ASTNode
import com.intellij.lang.folding.FoldingBuilderEx
import com.intellij.lang.folding.FoldingDescriptor
import com.intellij.openapi.editor.Document
import com.intellij.openapi.project.DumbAware
import com.intellij.psi.PsiElement
import com.intellij.psi.util.PsiTreeUtil
import org.ohxorud.breom.jetbrains.psi.BreomBlock
import org.ohxorud.breom.jetbrains.psi.BreomTypes
import java.util.ArrayList

class BreomFoldingBuilder : FoldingBuilderEx(), DumbAware {
    override fun buildFoldRegions(root: PsiElement, document: Document, quick: Boolean): Array<FoldingDescriptor> {
        val descriptors = ArrayList<FoldingDescriptor>()
        val blocks = PsiTreeUtil.findChildrenOfType(root, BreomBlock::class.java)

        for (block in blocks) {
            val textRange = block.textRange
            if (textRange.length > 2) {
                descriptors.add(FoldingDescriptor(block.node, textRange))
            }
        }

        return descriptors.toTypedArray()
    }

    override fun getPlaceholderText(node: ASTNode): String {
        return "{...}"
    }

    override fun isCollapsedByDefault(node: ASTNode): Boolean {
        return false
    }
}
