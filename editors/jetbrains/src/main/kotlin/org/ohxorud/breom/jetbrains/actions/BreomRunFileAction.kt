package org.ohxorud.breom.jetbrains.actions

import com.intellij.execution.ProgramRunnerUtil
import com.intellij.execution.RunManager
import com.intellij.execution.executors.DefaultRunExecutor
import com.intellij.openapi.actionSystem.ActionUpdateThread
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.project.DumbAwareAction
import com.intellij.openapi.ui.Messages
import org.ohxorud.breom.jetbrains.run.BreomRunConfiguration
import org.ohxorud.breom.jetbrains.run.BreomRunConfigurationType
import org.ohxorud.breom.jetbrains.run.BreomRunSupport

class BreomRunFileAction : DumbAwareAction() {
    private fun resolveTargetFile(e: AnActionEvent) =
        e.getData(CommonDataKeys.VIRTUAL_FILE)
            ?: e.getData(CommonDataKeys.PSI_FILE)?.virtualFile
            ?: e.project?.let { project ->
                FileEditorManager.getInstance(project).selectedFiles.firstOrNull()
            }

    override fun getActionUpdateThread(): ActionUpdateThread = ActionUpdateThread.BGT

    override fun update(e: AnActionEvent) {
        val file = resolveTargetFile(e)
        val enabled = file != null && !file.isDirectory && file.extension == "brm"
        e.presentation.isEnabledAndVisible = enabled
    }

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val file = resolveTargetFile(e)
        if (file == null || file.extension != "brm") {
            Messages.showErrorDialog(project, "Select a .brm file to run.", "Breom")
            return
        }

        val type = BreomRunConfigurationType.getInstance()
        val settings = RunManager.getInstance(project)
            .createConfiguration("Run ${file.name}", type.factory())
        val configuration = settings.configuration as BreomRunConfiguration
        configuration.runTarget = BreomRunConfiguration.TARGET_FILE
        configuration.filePath = file.path
        configuration.workingDirectory = BreomRunSupport.findNearestProjectRoot(file)?.path ?: (file.parent?.path ?: "")

        ProgramRunnerUtil.executeConfiguration(settings, DefaultRunExecutor.getRunExecutorInstance())
    }
}
