package org.ohxorud.breom.jetbrains.meta

import com.intellij.openapi.fileTypes.LanguageFileType
import org.ohxorud.breom.jetbrains.BreomIcons
import javax.swing.Icon

object BreomProjectFileType : LanguageFileType(BreomProjectLanguage) {
    override fun getName(): String = "Breom Project Manifest"
    override fun getDisplayName(): String = "Breom Project Manifest (project.breom)"

    override fun getDescription(): String = "Breom project manifest file"

    override fun getDefaultExtension(): String = "breom"

    override fun getIcon(): Icon = BreomIcons.FILE
}
