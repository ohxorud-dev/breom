package org.ohxorud.breom.jetbrains.run

import com.intellij.execution.lineMarker.RunLineMarkerContributor
import com.intellij.icons.AllIcons
import com.intellij.psi.PsiElement
import com.intellij.psi.util.PsiTreeUtil
import org.ohxorud.breom.jetbrains.actions.BreomRunFileAction
import org.ohxorud.breom.jetbrains.psi.BreomFunctionDecl

class BreomRunLineMarkerContributor : RunLineMarkerContributor() {
    override fun getInfo(element: PsiElement): Info? {
        val function = PsiTreeUtil.getParentOfType(element, BreomFunctionDecl::class.java, false) ?: return null
        if (function.name != "main") {
            return null
        }
        if (function.nameIdentifier != element) {
            return null
        }
        if (element.containingFile?.name != "main.brm") {
            return null
        }

        return Info(
            AllIcons.RunConfigurations.TestState.Run,
            arrayOf(BreomRunFileAction()),
        ) { "Run Breom main" }
    }
}
