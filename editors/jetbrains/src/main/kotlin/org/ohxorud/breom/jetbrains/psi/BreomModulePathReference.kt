package org.ohxorud.breom.jetbrains.psi

import com.intellij.codeInsight.lookup.LookupElementBuilder
import com.intellij.openapi.util.TextRange
import com.intellij.psi.*

class BreomModulePathPsiReference(element: PsiElement, textRange: TextRange) : PsiReferenceBase<PsiElement>(element, textRange), PsiPolyVariantReference {
    private val name: String = element.text.substring(textRange.startOffset, textRange.endOffset)

    override fun multiResolve(incompleteCode: Boolean): Array<ResolveResult> {
        val files = BreomUtil.findModuleFiles(myElement.project, name)
        val results = ArrayList<ResolveResult>()
        for (file in files) {
            results.add(PsiElementResolveResult(file))
        }
        return results.toArray(ResolveResult.EMPTY_ARRAY)
    }

    override fun resolve(): PsiElement? {
        val resolveResults = multiResolve(false)
        return if (resolveResults.isNotEmpty()) resolveResults[0].element else null
    }

    override fun getVariants(): Array<Any> {
        return BreomUtil.findAvailableModules(myElement.project)
            .map { module -> LookupElementBuilder.create(module) }
            .toTypedArray()
    }
}
