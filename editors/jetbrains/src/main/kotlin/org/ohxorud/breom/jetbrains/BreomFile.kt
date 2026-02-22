package org.ohxorud.breom.jetbrains

import com.intellij.extapi.psi.PsiFileBase
import com.intellij.openapi.fileTypes.FileType
import com.intellij.psi.FileViewProvider

class BreomFile(viewProvider: FileViewProvider) : PsiFileBase(viewProvider, BreomLanguage) {
    override fun getFileType(): FileType = BreomFileType
    override fun toString(): String = "Breom File"
}
