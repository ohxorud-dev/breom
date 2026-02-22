package org.ohxorud.breom.jetbrains

import com.intellij.openapi.fileTypes.LanguageFileType
import javax.swing.Icon

object BreomFileType : LanguageFileType(BreomLanguage) {
    override fun getName(): String = "Breom"
    override fun getDisplayName(): String = "Breom (.brm)"
    override fun getDescription(): String = "Breom language file"
    override fun getDefaultExtension(): String = "brm"
    override fun getIcon(): Icon = BreomIcons.FILE
}
