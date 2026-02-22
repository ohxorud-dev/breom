package org.ohxorud.breom.jetbrains.psi

import com.intellij.codeInsight.lookup.LookupElement
import com.intellij.codeInsight.lookup.LookupElementBuilder
import com.intellij.openapi.util.TextRange
import com.intellij.psi.*
import com.intellij.psi.util.PsiTreeUtil
import org.ohxorud.breom.jetbrains.BreomFile

class BreomReference(element: PsiElement, textRange: TextRange) : PsiReferenceBase<PsiElement>(element, textRange), PsiPolyVariantReference {
    private val name: String = element.text.substring(textRange.startOffset, textRange.endOffset)

    override fun multiResolve(incompleteCode: Boolean): Array<ResolveResult> {
        val definitions = LinkedHashSet<BreomNamedElement>()
        definitions.addAll(BreomUtil.findNamedElements(myElement, name))

        val project = myElement.project
        for (virtualFile in BreomUtil.findIndexedSymbolFiles(project, name)) {
            val file = PsiManager.getInstance(project).findFile(virtualFile) as? BreomFile ?: continue
            val symbols = PsiTreeUtil.findChildrenOfType(file, BreomNamedElement::class.java)
            for (symbol in symbols) {
                if (symbol.name == name) {
                    definitions.add(symbol)
                }
            }
        }

        val filtered = filterByUsageContext(definitions)

        val results = ArrayList<ResolveResult>()
        for (definition in filtered) {
            results.add(PsiElementResolveResult(definition))
        }
        return results.toArray(ResolveResult.EMPTY_ARRAY)
    }

    override fun resolve(): PsiElement? {
        val resolveResults = multiResolve(false)
        return if (resolveResults.size == 1) resolveResults[0].element else null
    }

    override fun getVariants(): Array<Any> {
        val definitions = BreomUtil.findNamedElementsInScope(myElement)
        val variants = ArrayList<LookupElement>()
        for (definition in definitions) {
            definition.name?.let {
                variants.add(LookupElementBuilder.create(definition).withIcon(definition.getIcon(0)).withTypeText(definition.containingFile.name))
            }
        }
        return variants.toTypedArray()
    }

    private fun filterByUsageContext(definitions: Set<BreomNamedElement>): List<BreomNamedElement> {
        if (definitions.isEmpty()) {
            return emptyList()
        }

        val expectsCallable = expectsCallableReference()
        if (!expectsCallable) {
            return definitions.toList()
        }

        val callable = definitions.filter { isCallableDeclaration(it) }
        return if (callable.isNotEmpty()) callable else definitions.toList()
    }

    private fun expectsCallableReference(): Boolean {
        val parentText = myElement.parent?.text ?: return false
        val marker = "$name("
        return parentText.contains(marker)
    }

    private fun isCallableDeclaration(element: BreomNamedElement): Boolean {
        val typeName = element.javaClass.simpleName
        return typeName.contains("FunctionDecl") || typeName.contains("MethodDecl")
    }
}
