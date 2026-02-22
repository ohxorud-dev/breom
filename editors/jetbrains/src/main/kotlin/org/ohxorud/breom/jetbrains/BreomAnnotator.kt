package org.ohxorud.breom.jetbrains

import com.intellij.lang.annotation.AnnotationHolder
import com.intellij.lang.annotation.Annotator
import com.intellij.lang.annotation.HighlightSeverity
import com.intellij.openapi.editor.DefaultLanguageHighlighterColors
import com.intellij.psi.PsiElement
import org.ohxorud.breom.jetbrains.psi.BreomFunctionDecl
import org.ohxorud.breom.jetbrains.psi.BreomParam
import org.ohxorud.breom.jetbrains.psi.BreomStructDecl
import org.ohxorud.breom.jetbrains.psi.BreomReferenceExpression
import com.intellij.psi.PsiPolyVariantReference

class BreomAnnotator : Annotator {
    override fun annotate(element: PsiElement, holder: AnnotationHolder) {
        val fileName = element.containingFile?.name
        if (fileName.equals("project.breom", ignoreCase = true)
            || fileName.equals("lock.breom", ignoreCase = true)
        ) {
            return
        }

        if (element is BreomFunctionDecl) {
            val identifier = element.nameIdentifier
            holder.newSilentAnnotation(HighlightSeverity.INFORMATION)
                .range(identifier.textRange)
                .textAttributes(DefaultLanguageHighlighterColors.FUNCTION_DECLARATION)
                .create()
        } else if (element is BreomStructDecl) {
            val identifier = element.typeIdentifier
            holder.newSilentAnnotation(HighlightSeverity.INFORMATION)
                .range(identifier.textRange)
                .textAttributes(DefaultLanguageHighlighterColors.CLASS_NAME)
                .create()
        } else if (element is BreomParam) {
            val identifier = element.nameIdentifier
            holder.newSilentAnnotation(HighlightSeverity.INFORMATION)
                .range(identifier.textRange)
                .textAttributes(DefaultLanguageHighlighterColors.PARAMETER)
                .create()
        } else if (element is BreomReferenceExpression) {
            val reference = element.reference
            if (reference is PsiPolyVariantReference) {
                val results = reference.multiResolve(false)
                if (results.isEmpty()) {
                    holder.newAnnotation(
                        HighlightSeverity.ERROR,
                        "Cannot resolve symbol `${element.text}`. Add an import or declare it in this module."
                    )
                        .range(element.textRange)
                        .create()
                }
            }
        }
    }
}
