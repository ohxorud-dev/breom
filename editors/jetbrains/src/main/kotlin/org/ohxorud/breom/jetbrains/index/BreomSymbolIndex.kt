package org.ohxorud.breom.jetbrains.index

import com.intellij.util.indexing.DataIndexer
import com.intellij.util.indexing.DefaultFileTypeSpecificInputFilter
import com.intellij.util.indexing.FileBasedIndex
import com.intellij.util.indexing.FileContent
import com.intellij.util.indexing.ID
import com.intellij.util.indexing.ScalarIndexExtension
import com.intellij.util.io.EnumeratorStringDescriptor
import org.ohxorud.breom.jetbrains.BreomFileType

class BreomSymbolIndex : ScalarIndexExtension<String>() {
    override fun getName(): ID<String, Void> = NAME

    override fun getIndexer(): DataIndexer<String, Void, FileContent> = DataIndexer { input ->
        if (isMetaFile(input.fileName)) {
            return@DataIndexer emptyMap()
        }
        val out = LinkedHashMap<String, Void?>()
        for (match in symbolRegex.findAll(input.contentAsText)) {
            val name = match.groupValues.getOrNull(2).orEmpty()
            if (name.isNotBlank()) {
                out[name] = null
            }
        }
        out
    }

    override fun getKeyDescriptor() = EnumeratorStringDescriptor.INSTANCE

    override fun getInputFilter(): FileBasedIndex.InputFilter =
        DefaultFileTypeSpecificInputFilter(BreomFileType)

    override fun dependsOnFileContent(): Boolean = true

    override fun getVersion(): Int = 1

    private fun isMetaFile(fileName: String): Boolean {
        return fileName.equals("project.breom", ignoreCase = true)
            || fileName.equals("lock.breom", ignoreCase = true)
    }

    companion object {
        val NAME: ID<String, Void> = ID.create("breom.symbol.index")
        private val symbolRegex = Regex(
            "(?m)^\\s*(?:pub\\s+)?(fn|struct|interface|enum|define)\\s+([A-Za-z_][A-Za-z0-9_]*)"
        )
    }
}
