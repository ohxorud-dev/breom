package org.ohxorud.breom.jetbrains.meta

import com.intellij.openapi.fileTypes.LanguageFileType
import org.ohxorud.breom.jetbrains.BreomIcons
import javax.swing.Icon

object BreomLockFileType : LanguageFileType(BreomLockLanguage) {
    override fun getName(): String = "Breom Lockfile"
    override fun getDisplayName(): String = "Breom Lockfile (lock.breom)"

    override fun getDescription(): String = "Breom dependency lock file"

    override fun getDefaultExtension(): String = "breom"

    override fun getIcon(): Icon = BreomIcons.FILE
}
