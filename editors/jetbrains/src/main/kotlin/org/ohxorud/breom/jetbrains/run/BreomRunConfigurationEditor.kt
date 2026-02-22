package org.ohxorud.breom.jetbrains.run

import com.intellij.openapi.fileChooser.FileChooserDescriptorFactory
import com.intellij.openapi.options.SettingsEditor
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.ComboBox
import com.intellij.openapi.ui.TextFieldWithBrowseButton
import com.intellij.ui.components.JBLabel
import com.intellij.util.ui.FormBuilder
import java.awt.event.ItemEvent
import javax.swing.JComponent
import javax.swing.JPanel

class BreomRunConfigurationEditor(project: Project) : SettingsEditor<BreomRunConfiguration>() {
    private val targetCombo = ComboBox(arrayOf(BreomRunConfiguration.TARGET_PROJECT, BreomRunConfiguration.TARGET_FILE))
    private val fileLabel = JBLabel("File")
    private val fileField = TextFieldWithBrowseButton()
    private val workDirField = TextFieldWithBrowseButton()
    private val breomPathField = TextFieldWithBrowseButton()
    private val panel: JPanel

    init {
        fileField.addBrowseFolderListener(
            "Breom file",
            "Select .brm file",
            project,
            FileChooserDescriptorFactory.createSingleFileDescriptor("brm"),
        )
        workDirField.addBrowseFolderListener(
            "Working directory",
            "Select working directory",
            project,
            FileChooserDescriptorFactory.createSingleFolderDescriptor(),
        )
        breomPathField.addBrowseFolderListener(
            "Breom executable",
            "Select Breom executable",
            project,
            FileChooserDescriptorFactory.createSingleFileNoJarsDescriptor(),
        )

        targetCombo.addItemListener {
            if (it.stateChange == ItemEvent.SELECTED) {
                updateUiState()
            }
        }

        panel = FormBuilder.createFormBuilder()
            .addLabeledComponent(JBLabel("Target"), targetCombo, 1, false)
            .addLabeledComponent(fileLabel, fileField, 1, false)
            .addLabeledComponent(JBLabel("Working directory"), workDirField, 1, false)
            .addLabeledComponent(JBLabel("Breom executable"), breomPathField, 1, false)
            .addComponentFillVertically(JPanel(), 0)
            .panel

        updateUiState()
    }

    override fun resetEditorFrom(configuration: BreomRunConfiguration) {
        targetCombo.selectedItem = configuration.runTarget
        fileField.text = configuration.filePath
        workDirField.text = configuration.workingDirectory
        breomPathField.text = configuration.breomPath
        updateUiState()
    }

    override fun applyEditorTo(configuration: BreomRunConfiguration) {
        configuration.runTarget = targetCombo.selectedItem as String
        configuration.filePath = fileField.text.trim()
        configuration.workingDirectory = workDirField.text.trim()
        configuration.breomPath = breomPathField.text.trim()
    }

    override fun createEditor(): JComponent = panel

    private fun updateUiState() {
        val isFile = targetCombo.selectedItem == BreomRunConfiguration.TARGET_FILE
        fileLabel.isVisible = isFile
        fileField.isVisible = isFile
        fileField.isEnabled = isFile
    }
}
