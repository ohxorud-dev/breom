package org.ohxorud.breom.jetbrains.run

import com.intellij.ide.util.PropertiesComponent
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import org.ohxorud.breom.jetbrains.settings.BreomSettingsService
import org.ohxorud.breom.jetbrains.settings.BreomSettingsConfigurable
import java.nio.file.Files
import java.nio.file.Path
import kotlin.io.path.exists
import kotlin.io.path.isDirectory

object BreomRunSupport {
    private val executableNames = listOf("breom", "breom.exe")

    fun resolveExecutable(project: Project, configuredPath: String): String? {
        val explicit = configuredPath.trim()
        if (explicit.isNotEmpty()) {
            return explicit
        }

        val homeFromProperties = PropertiesComponent.getInstance().getValue(BreomSettingsConfigurable.BREOM_HOME_KEY, "")
        val home = homeFromProperties
            .ifBlank { BreomSettingsService.getInstanceOrNull()?.breomHomePath.orEmpty() }
            .ifBlank { System.getenv("BREOM_HOME").orEmpty() }
        val homePath = home.trim()
        if (homePath.isNotEmpty()) {
            val fromHome = resolveFromHome(Path.of(homePath))
            if (fromHome != null) {
                return fromHome
            }
        }

        return null
    }

    private fun resolveFromHome(home: Path): String? {
        if (!home.exists() || !home.isDirectory()) {
            return null
        }

        val candidates = executableNames.map { name -> home.resolve("bin").resolve(name) }

        return candidates.firstOrNull { Files.isRegularFile(it) }?.toString()
    }

    fun findNearestProjectRoot(file: VirtualFile?): VirtualFile? {
        var current = if (file?.isDirectory == true) file else file?.parent
        while (current != null) {
            if (current.findChild("project.breom") != null) {
                return current
            }
            current = current.parent
        }
        return null
    }
}
