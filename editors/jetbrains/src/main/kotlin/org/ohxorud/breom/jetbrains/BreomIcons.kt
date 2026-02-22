package org.ohxorud.breom.jetbrains

import com.intellij.openapi.util.IconLoader
import com.intellij.util.IconUtil

object BreomIcons {
    val FILE = IconUtil.scale(IconLoader.getIcon("/icon.svg", BreomIcons::class.java), null, 16f / 512f)
}
