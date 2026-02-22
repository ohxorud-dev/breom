package org.ohxorud.breom.jetbrains.meta

import com.intellij.lang.annotation.AnnotationHolder
import com.intellij.lang.annotation.Annotator
import com.intellij.lang.annotation.HighlightSeverity
import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile

class BreomLockAnnotator : Annotator {
    override fun annotate(element: PsiElement, holder: AnnotationHolder) {
        if (element !is PsiFile) {
            return
        }
        if (!element.name.equals("lock.breom", ignoreCase = true)) {
            return
        }

        val content = element.text
        val diagnostics = BreomMetaValidators.validateLockFile(content)
        for (diag in diagnostics) {
            val severity = when (diag.severity) {
                MetaSeverity.ERROR -> HighlightSeverity.ERROR
                MetaSeverity.WARNING -> HighlightSeverity.WARNING
            }
            holder.newAnnotation(severity, diag.message)
                .range(TextRange(0, minOf(content.length, 1)))
                .create()
        }
    }
}
