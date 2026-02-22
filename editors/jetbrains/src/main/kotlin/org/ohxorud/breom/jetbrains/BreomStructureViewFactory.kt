package org.ohxorud.breom.jetbrains

import com.intellij.ide.projectView.PresentationData
import com.intellij.ide.structureView.*
import com.intellij.ide.util.treeView.smartTree.SortableTreeElement
import com.intellij.ide.util.treeView.smartTree.Sorter
import com.intellij.ide.util.treeView.smartTree.TreeElement
import com.intellij.lang.PsiStructureViewFactory
import com.intellij.navigation.ItemPresentation
import com.intellij.openapi.editor.Editor
import com.intellij.psi.NavigatablePsiElement
import com.intellij.psi.PsiFile
import org.ohxorud.breom.jetbrains.psi.BreomFunctionDecl
import org.ohxorud.breom.jetbrains.psi.BreomStructDecl

class BreomStructureViewFactory : PsiStructureViewFactory {
    override fun getStructureViewBuilder(psiFile: PsiFile): StructureViewBuilder? {
        if (psiFile !is BreomFile) return null
        return object : TreeBasedStructureViewBuilder() {
            override fun createStructureViewModel(editor: Editor?): StructureViewModel {
                return BreomStructureViewModel(psiFile)
            }
        }
    }
}

class BreomStructureViewModel(psiFile: PsiFile) : StructureViewModelBase(psiFile, BreomStructureViewElement(psiFile)),
    StructureViewModel.ElementInfoProvider {

    override fun getSorters(): Array<Sorter> = arrayOf(Sorter.ALPHA_SORTER)

    override fun isAlwaysShowsPlus(element: StructureViewTreeElement): Boolean {
        return false
    }

    override fun isAlwaysLeaf(element: StructureViewTreeElement): Boolean {
        return element.value is BreomFunctionDecl || element.value is BreomStructDecl
    }
}

class BreomStructureViewElement(private val element: NavigatablePsiElement) : StructureViewTreeElement, SortableTreeElement {

    override fun getValue(): Any = element

    override fun navigate(requestFocus: Boolean) {
        element.navigate(requestFocus)
    }

    override fun canNavigate(): Boolean = element.canNavigate()

    override fun canNavigateToSource(): Boolean = element.canNavigateToSource()

    override fun getAlphaSortKey(): String = element.name ?: ""

    override fun getPresentation(): ItemPresentation {
        return element.presentation ?: PresentationData()
    }

    override fun getChildren(): Array<TreeElement> {
        if (element is BreomFile) {
            val properties = ArrayList<TreeElement>()
            element.children.forEach { child ->
                if (child is BreomFunctionDecl) {
                    properties.add(BreomStructureViewElement(child as NavigatablePsiElement))
                } else if (child is BreomStructDecl) {
                    properties.add(BreomStructureViewElement(child as NavigatablePsiElement))
                }
            }
            return properties.toTypedArray()
        }
        return emptyArray()
    }
}
