package org.ohxorud.breom.jetbrains.meta

import com.intellij.lang.annotation.AnnotationHolder
import com.intellij.lang.annotation.Annotator
import com.intellij.lang.annotation.HighlightSeverity
import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile

class BreomProjectAnnotator : Annotator {
    override fun annotate(element: PsiElement, holder: AnnotationHolder) {
        if (element !is PsiFile) {
            return
        }
        if (!element.name.equals("project.breom", ignoreCase = true)) {
            return
        }

        val content = element.text
        val projectRoot = element.virtualFile?.parent?.path
        val diagnostics = BreomMetaValidators.validateProjectManifest(content, projectRoot)
        val lineRanges = lineRanges(content)

        for (diag in diagnostics) {
            val range = lineRanges.getOrNull(diag.line) ?: TextRange(0, minOf(1, content.length))
            val severity = when (diag.severity) {
                MetaSeverity.ERROR -> HighlightSeverity.ERROR
                MetaSeverity.WARNING -> HighlightSeverity.WARNING
            }
            holder.newAnnotation(severity, diag.message)
                .range(range)
                .create()
        }
    }

    private fun lineRanges(content: String): List<TextRange> {
        val ranges = mutableListOf<TextRange>()
        var start = 0
        for (line in content.lines()) {
            val end = start + line.length
            ranges += TextRange(start, end)
            start = end + 1
        }
        if (ranges.isEmpty()) {
            ranges += TextRange(0, 0)
        }
        return ranges
    }
}
