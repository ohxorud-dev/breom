package org.ohxorud.breom.jetbrains.actions

import com.intellij.execution.ProgramRunnerUtil
import com.intellij.execution.RunManager
import com.intellij.execution.executors.DefaultRunExecutor
import com.intellij.openapi.actionSystem.ActionUpdateThread
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.project.DumbAwareAction
import com.intellij.openapi.ui.Messages
import org.ohxorud.breom.jetbrains.run.BreomRunConfiguration
import org.ohxorud.breom.jetbrains.run.BreomRunConfigurationType
import org.ohxorud.breom.jetbrains.run.BreomRunSupport

class BreomRunProjectAction : DumbAwareAction() {
    override fun getActionUpdateThread(): ActionUpdateThread = ActionUpdateThread.BGT

    override fun update(e: AnActionEvent) {
        e.presentation.isEnabledAndVisible = e.project != null
    }

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val selected = e.getData(CommonDataKeys.VIRTUAL_FILE)
            ?: e.getData(CommonDataKeys.PSI_FILE)?.virtualFile

        val root = when {
            selected?.name == "project.breom" -> selected.parent
            else -> BreomRunSupport.findNearestProjectRoot(selected)
        }
        val workingDir = root?.path ?: project.basePath

        if (workingDir.isNullOrBlank()) {
            Messages.showErrorDialog(project, "Cannot determine project directory.", "Breom")
            return
        }

        val type = BreomRunConfigurationType.getInstance()
        val settings = RunManager.getInstance(project)
            .createConfiguration("Run Breom Project", type.factory())
        val configuration = settings.configuration as BreomRunConfiguration
        configuration.runTarget = BreomRunConfiguration.TARGET_PROJECT
        configuration.workingDirectory = workingDir

        ProgramRunnerUtil.executeConfiguration(settings, DefaultRunExecutor.getRunExecutorInstance())
    }
}
