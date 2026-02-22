package org.ohxorud.breom.jetbrains.psi

import com.intellij.patterns.PlatformPatterns
import com.intellij.psi.PsiReferenceContributor
import com.intellij.psi.PsiReferenceRegistrar
import com.intellij.psi.PsiReferenceProvider
import com.intellij.psi.PsiReference
import com.intellij.util.ProcessingContext

class BreomModulePathReferenceContributor : PsiReferenceContributor() {
    override fun registerReferenceProviders(registrar: PsiReferenceRegistrar) {
        registrar.registerReferenceProvider(
            PlatformPatterns.psiElement(BreomModulePath::class.java),
            object : PsiReferenceProvider() {
                override fun getReferencesByElement(
                    element: com.intellij.psi.PsiElement,
                    context: ProcessingContext,
                ): Array<PsiReference> {
                    if (element.text.isBlank()) {
                        return PsiReference.EMPTY_ARRAY
                    }
                    return arrayOf(
                        BreomModulePathPsiReference(
                            element,
                            com.intellij.openapi.util.TextRange(0, element.textLength),
                        )
                    )
                }
            }
        )
    }
}
