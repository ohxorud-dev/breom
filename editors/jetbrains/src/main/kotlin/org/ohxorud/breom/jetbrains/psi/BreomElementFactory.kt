package org.ohxorud.breom.jetbrains.psi

import org.ohxorud.breom.jetbrains.psi.BreomTypes
import com.intellij.openapi.project.Project
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFileFactory
import com.intellij.psi.util.PsiTreeUtil
import org.ohxorud.breom.jetbrains.BreomFileType
import org.ohxorud.breom.jetbrains.BreomFile

object BreomElementFactory {
    fun createIdentifier(project: Project, name: String): PsiElement? {
        val file = createFile(project, "define $name = 0")
        val defineDecl = PsiTreeUtil.findChildOfType(file, BreomDefineDecl::class.java)
        return defineDecl?.nameIdentifier
    }

    fun createFile(project: Project, text: String, name: String = "dummy.brm"): BreomFile {
        return PsiFileFactory.getInstance(project).createFileFromText(name, BreomFileType, text) as BreomFile
    }
}
