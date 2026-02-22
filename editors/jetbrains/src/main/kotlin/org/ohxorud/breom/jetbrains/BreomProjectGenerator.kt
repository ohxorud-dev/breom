package org.ohxorud.breom.jetbrains

import com.intellij.ide.util.projectWizard.SettingsStep
import com.intellij.facet.ui.ValidationResult
import com.intellij.openapi.module.Module
import com.intellij.openapi.ui.ValidationInfo
import com.intellij.openapi.project.Project
import com.intellij.openapi.application.WriteAction
import com.intellij.openapi.vfs.VfsUtil
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.DirectoryProjectGenerator
import com.intellij.platform.ProjectGeneratorPeer
import com.intellij.platform.WebProjectGenerator
import com.intellij.openapi.ui.TextFieldWithBrowseButton
import javax.swing.Icon
import javax.swing.JComponent
import javax.swing.JPanel

class BreomProjectGenerator : DirectoryProjectGenerator<BreomProjectGenerator.Settings> {
    class Settings

    override fun getName(): String = "Breom"

    override fun isPrimaryGenerator(): Boolean = true

    override fun getDescription(): String = "Create a new Breom project"

    override fun getLogo(): Icon = BreomIcons.FILE

    override fun createPeer(): ProjectGeneratorPeer<Settings> {
        return object : ProjectGeneratorPeer<Settings> {
            private val panel = JPanel()

            override fun getComponent(
                mySettingsStep: TextFieldWithBrowseButton,
                checkValid: Runnable,
            ): JComponent = panel

            override fun buildUI(settingsStep: SettingsStep) {}

            override fun getSettings(): Settings = Settings()

            override fun validate(): ValidationInfo? = null

            override fun isBackgroundJobRunning(): Boolean = false

            override fun addSettingsStateListener(listener: WebProjectGenerator.SettingsStateListener) {}
        }
    }

    override fun validate(baseDirPath: String): ValidationResult = ValidationResult.OK

    override fun generateProject(
        project: Project,
        baseDir: VirtualFile,
        settings: Settings,
        module: Module,
    ) {
        WriteAction.run<Throwable> {
            val projectFile = baseDir.findChild("project.breom")
                ?: baseDir.createChildData(this, "project.breom")
            val moduleName = baseDir.name.ifBlank { "app" }
            VfsUtil.saveText(projectFile, "breom 0.1.0\npackage $moduleName\n")

            val mainFile = baseDir.findChild("main.brm")
                ?: baseDir.createChildData(this, "main.brm")
            VfsUtil.saveText(
                mainFile,
                "fn main() Int {\n    println(\"Hello, Breom!\")\n    return 0\n}\n",
            )
        }
    }
}
