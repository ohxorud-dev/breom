package org.ohxorud.breom.jetbrains.settings

import com.intellij.ide.util.PropertiesComponent
import com.intellij.openapi.fileChooser.FileChooserDescriptorFactory
import com.intellij.openapi.options.Configurable
import com.intellij.openapi.ui.TextFieldWithBrowseButton
import com.intellij.ui.components.JBLabel
import com.intellij.util.ui.FormBuilder
import javax.swing.JComponent
import javax.swing.JPanel

class BreomSettingsConfigurable : Configurable {
    private var breomHomeField: TextFieldWithBrowseButton? = null
    private var panel: JPanel? = null

    private fun currentBreomHome(): String {
        return PropertiesComponent.getInstance().getValue(BREOM_HOME_KEY, "")
    }

    override fun getDisplayName(): String = "Breom"

    override fun createComponent(): JComponent {
        if (panel == null) {
            val field = TextFieldWithBrowseButton()
            field.addBrowseFolderListener(
                "BREOM_HOME",
                "Select BREOM_HOME directory (std resolves from BREOM_HOME/<version>/std; fallback: <version>/src, std, src)",
                null,
                FileChooserDescriptorFactory.createSingleFolderDescriptor(),
            )
            field.text = currentBreomHome()
            breomHomeField = field

            panel = FormBuilder.createFormBuilder()
                .addLabeledComponent(JBLabel("BREOM_HOME"), field, 1, false)
                .addComponentFillVertically(JPanel(), 0)
                .panel
        }
        return panel as JPanel
    }

    override fun isModified(): Boolean {
        val fieldText = breomHomeField?.text?.trim().orEmpty()
        return fieldText != currentBreomHome()
    }

    override fun apply() {
        val value = breomHomeField?.text?.trim().orEmpty()
        PropertiesComponent.getInstance().setValue(BREOM_HOME_KEY, value, "")
        BreomSettingsService.getInstanceOrNull()?.breomHomePath = value
    }

    override fun reset() {
        breomHomeField?.text = currentBreomHome()
    }

    override fun disposeUIResources() {
        breomHomeField = null
        panel = null
    }

    companion object {
        const val BREOM_HOME_KEY = "breom.home.path"
    }
}
