package org.ohxorud.breom.jetbrains.psi

import com.intellij.openapi.project.Project
import com.intellij.openapi.project.guessProjectDir
import com.intellij.psi.PsiFile
import com.intellij.psi.PsiManager
import com.intellij.psi.search.FileTypeIndex
import com.intellij.psi.search.GlobalSearchScope
import com.intellij.psi.util.PsiTreeUtil
import com.intellij.psi.PsiElement
import com.intellij.util.indexing.FileBasedIndex
import org.ohxorud.breom.jetbrains.BreomFile
import org.ohxorud.breom.jetbrains.BreomFileType
import org.ohxorud.breom.jetbrains.index.BreomModulePathIndex
import org.ohxorud.breom.jetbrains.index.BreomSymbolIndex
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VfsUtilCore
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.vfs.VirtualFileVisitor
import java.nio.file.Files
import java.nio.file.Path as JPath
import kotlin.io.path.Path
import kotlin.io.path.exists
import kotlin.io.path.isDirectory
import org.ohxorud.breom.jetbrains.psi.BreomDependDecl
import org.ohxorud.breom.jetbrains.psi.BreomModuleDecl

object BreomUtil {

    private var builtInFiles: List<BreomFile>? = null
    private val semverPattern = Regex("""^(\\d+)\\.(\\d+)\\.(\\d+)$""")

    private data class SemVer(
        val major: Int,
        val minor: Int,
        val patch: Int,
    ) : Comparable<SemVer> {
        override fun compareTo(other: SemVer): Int {
            if (major != other.major) {
                return major.compareTo(other.major)
            }
            if (minor != other.minor) {
                return minor.compareTo(other.minor)
            }
            return patch.compareTo(other.patch)
        }
    }

    private fun getBuiltInFiles(project: Project): List<BreomFile> {
        if (builtInFiles != null && builtInFiles!!.all { it.isValid }) {
            return builtInFiles!!
        }

        val result = ArrayList<BreomFile>()
        val stdDir = resolveStdSrcDir() ?: return emptyList()

        VfsUtilCore.visitChildrenRecursively(stdDir, object : VirtualFileVisitor<Unit>() {
            override fun visitFile(file: VirtualFile): Boolean {
                if (!file.isDirectory && file.extension == "brm") {
                    val psiFile = PsiManager.getInstance(project).findFile(file) as? BreomFile
                    if (psiFile != null) {
                        result.add(psiFile)
                    }
                }
                return true
            }
        })

        builtInFiles = result
        return result
    }

    private fun resolveStdSrcDir(): VirtualFile? {
        val breomHome = System.getenv("BREOM_HOME")
        if (breomHome.isNullOrBlank()) {
            return null
        }

        val breomHomePath = Path(breomHome)
        val stdSrcPath =
            resolveVersionedStdSrcPath(breomHomePath)
                ?: breomHomePath.resolve("std").takeIf { it.exists() && it.isDirectory() }
                ?: breomHomePath.resolve("src")
        if (!stdSrcPath.exists() || !stdSrcPath.isDirectory()) {
            return null
        }

        val file = LocalFileSystem.getInstance().findFileByNioFile(stdSrcPath)
        if (file != null && file.isValid && file.isDirectory) {
            return file
        }

        return null
    }

    private fun resolveVersionedStdSrcPath(breomHomePath: JPath): JPath? {
        if (!breomHomePath.exists() || !breomHomePath.isDirectory()) {
            return null
        }

        var best: Pair<SemVer, JPath>? = null
        Files.newDirectoryStream(breomHomePath).use { entries ->
            for (entry in entries) {
                if (!entry.isDirectory()) {
                    continue
                }

                val version = parseSemVer(entry.fileName.toString()) ?: continue
                val candidate = when {
                    entry.resolve("std").exists() && entry.resolve("std").isDirectory() -> {
                        entry.resolve("std")
                    }
                    entry.resolve("src").exists() && entry.resolve("src").isDirectory() -> {
                        entry.resolve("src")
                    }
                    else -> continue
                }

                if (best == null || version > best!!.first) {
                    best = version to candidate
                }
            }
        }

        return best?.second
    }

    private fun parseSemVer(value: String): SemVer? {
        val match = semverPattern.matchEntire(value) ?: return null
        return SemVer(
            major = match.groupValues[1].toIntOrNull() ?: return null,
            minor = match.groupValues[2].toIntOrNull() ?: return null,
            patch = match.groupValues[3].toIntOrNull() ?: return null,
        )
    }


    fun findNamedElements(element: PsiElement, name: String): List<BreomNamedElement> {
        val result = ArrayList<BreomNamedElement>()
        val project = element.project
        val currentFile = element.containingFile as? BreomFile ?: return emptyList()


        val localElements = PsiTreeUtil.findChildrenOfType(currentFile, BreomNamedElement::class.java)
        for (e in localElements) {
            if (name == e.name) {
                result.add(e)
            }
        }



        val imports = PsiTreeUtil.findChildrenOfType(currentFile, BreomDependDecl::class.java)
        for (imp in imports) {
            val importPath = imp.modulePath.text





            val targetFiles = findFilesByImport(project, importPath)
             for (file in targetFiles) {
                val importedElements = PsiTreeUtil.findChildrenOfType(file, BreomNamedElement::class.java)
                for (e in importedElements) {
                    if (name == e.name && e != element) {


                         result.add(e)
                    }
                }
            }
        }


        val indexedFiles = FileBasedIndex.getInstance().getContainingFiles(
            BreomSymbolIndex.NAME,
            name,
            GlobalSearchScope.projectScope(project)
        )
        for (virtualFile in indexedFiles) {
            val file = PsiManager.getInstance(project).findFile(virtualFile) as? BreomFile ?: continue
            val symbols = PsiTreeUtil.findChildrenOfType(file, BreomNamedElement::class.java)
            for (e in symbols) {
                if (name == e.name && e.containingFile != currentFile) {
                    result.add(e)
                }
            }
        }


        val builtIns = getBuiltInFiles(project)
        for (file in builtIns) {
            val builtinElements = PsiTreeUtil.findChildrenOfType(file, BreomNamedElement::class.java)
            for (e in builtinElements) {
                if (name == e.name) {
                    result.add(e)
                }
            }
        }

        return result
    }





    private fun findFilesByImport(project: Project, importPath: String): List<BreomFile> {
        val matchingFiles = ArrayList<BreomFile>()
        val virtualFiles = FileBasedIndex.getInstance().getContainingFiles(
            BreomModulePathIndex.NAME,
            importPath,
            GlobalSearchScope.projectScope(project)
        )

        for (virtualFile in virtualFiles) {
            val file = PsiManager.getInstance(project).findFile(virtualFile) as? BreomFile ?: continue

            val packageName = inferPackageName(project, file)
            if (packageName == importPath) {
                matchingFiles.add(file)
            }
        }

        if (matchingFiles.isNotEmpty()) {
            return matchingFiles
        }

        val fallbackFiles = FileTypeIndex.getFiles(BreomFileType, GlobalSearchScope.allScope(project))
        for (virtualFile in fallbackFiles) {
            val file = PsiManager.getInstance(project).findFile(virtualFile) as? BreomFile ?: continue
            val packageName = inferPackageName(project, file)
            if (packageName == importPath) {
                matchingFiles.add(file)
            }
        }

        if (matchingFiles.isNotEmpty()) {
            return matchingFiles
        }

        matchingFiles.addAll(findStdFilesByImport(project, importPath))
        return matchingFiles
    }

    private fun findStdFilesByImport(project: Project, importPath: String): List<BreomFile> {
        val stdRoot = resolveStdSrcDir() ?: return emptyList()
        val out = ArrayList<BreomFile>()

        VfsUtilCore.visitChildrenRecursively(stdRoot, object : VirtualFileVisitor<Unit>() {
            override fun visitFile(file: VirtualFile): Boolean {
                if (!file.isDirectory && file.extension == "brm") {
                    val psi = PsiManager.getInstance(project).findFile(file) as? BreomFile ?: return true
                    val moduleDecl = PsiTreeUtil.findChildOfType(psi, BreomModuleDecl::class.java)
                    val moduleName = moduleDecl?.modulePath?.text ?: inferStdPackageFromPath(stdRoot, file)
                    if (moduleName == importPath) {
                        out.add(psi)
                    }
                }
                return true
            }
        })

        return out
    }

    private fun inferStdPackageFromPath(stdRoot: VirtualFile, file: VirtualFile): String {
        val parent = file.parent ?: return ""
        val relative = VfsUtilCore.getRelativePath(parent, stdRoot, '/') ?: return ""
        return relative.replace('/', '.')
    }

    fun findModuleFiles(project: Project, modulePath: String): List<BreomFile> {
        val matchingFiles = LinkedHashSet<BreomFile>()
        matchingFiles.addAll(findFilesByImport(project, modulePath))

        val builtIns = getBuiltInFiles(project)
        for (file in builtIns) {
            val moduleDecl = PsiTreeUtil.findChildOfType(file, BreomModuleDecl::class.java)
            if (moduleDecl?.modulePath?.text == modulePath) {
                matchingFiles.add(file)
            }
        }

        return matchingFiles.toList()
    }

    fun findAvailableModules(project: Project): List<String> {
        val modules = LinkedHashSet<String>()

        val virtualFiles = FileTypeIndex.getFiles(BreomFileType, GlobalSearchScope.allScope(project))
        for (virtualFile in virtualFiles) {
            val file = PsiManager.getInstance(project).findFile(virtualFile) as? BreomFile ?: continue
            val moduleName = inferPackageName(project, file)
            if (moduleName.isNotBlank()) {
                modules.add(moduleName)
            }
        }

        modules.addAll(discoverStdModulesFromFilesystem(project))

        return modules.sorted()
    }

    private fun discoverStdModulesFromFilesystem(project: Project): Set<String> {
        val stdRoot = resolveStdSrcDir() ?: return emptySet()
        val modules = LinkedHashSet<String>()

        VfsUtilCore.visitChildrenRecursively(stdRoot, object : VirtualFileVisitor<Unit>() {
            override fun visitFile(file: VirtualFile): Boolean {
                if (!file.isDirectory && file.extension == "brm") {
                    val relative = VfsUtilCore.getRelativePath(file.parent ?: return true, stdRoot, '/')
                    if (!relative.isNullOrBlank()) {
                        modules.add(relative.replace('/', '.'))
                    }
                }
                return true
            }
        })

        return modules
    }



    fun findNamedElements(project: Project): List<BreomNamedElement> {





        val result = ArrayList<BreomNamedElement>()


        val builtIns = getBuiltInFiles(project)
        for (file in builtIns) {
            result.addAll(PsiTreeUtil.findChildrenOfType(file, BreomNamedElement::class.java))
        }


        val virtualFiles = FileTypeIndex.getFiles(BreomFileType, GlobalSearchScope.allScope(project))
        for (virtualFile in virtualFiles) {
            val breomFile = PsiManager.getInstance(project).findFile(virtualFile) as? BreomFile ?: continue
            val namedElements = PsiTreeUtil.findChildrenOfType(breomFile, BreomNamedElement::class.java)
            result.addAll(namedElements)
        }
        return result
    }

    fun findIndexedSymbolFiles(project: Project, symbolName: String): Collection<VirtualFile> {
        val indexed = FileBasedIndex.getInstance().getContainingFiles(
            BreomSymbolIndex.NAME,
            symbolName,
            GlobalSearchScope.projectScope(project)
        )
        return if (indexed.isNotEmpty()) indexed else FileTypeIndex.getFiles(BreomFileType, GlobalSearchScope.allScope(project))
    }


     fun findNamedElementsInScope(element: PsiElement): List<BreomNamedElement> {
        val result = ArrayList<BreomNamedElement>()
        val project = element.project
        val currentFile = element.containingFile as? BreomFile ?: return emptyList()


        result.addAll(PsiTreeUtil.findChildrenOfType(currentFile, BreomNamedElement::class.java))


        val builtIns = getBuiltInFiles(project)
        for (file in builtIns) {
            result.addAll(PsiTreeUtil.findChildrenOfType(file, BreomNamedElement::class.java))
        }


        val imports = PsiTreeUtil.findChildrenOfType(currentFile, BreomDependDecl::class.java)
        for (imp in imports) {
            val importPath = imp.modulePath.text
            val targetFiles = findFilesByImport(project, importPath)
              for (file in targetFiles) {
                  result.addAll(PsiTreeUtil.findChildrenOfType(file, BreomNamedElement::class.java))
             }
        }

         return result
      }

    private fun inferPackageName(project: Project, file: BreomFile): String {
        val moduleDecl = PsiTreeUtil.findChildOfType(file, BreomModuleDecl::class.java)
        if (moduleDecl != null) {
            return moduleDecl.modulePath.text
        }

        val virtualFile = file.virtualFile ?: return ""

        val stdRoot = resolveStdSrcDir()
        if (stdRoot != null && VfsUtilCore.isAncestor(stdRoot, virtualFile, false)) {
            val relativeParent = VfsUtilCore.getRelativePath(virtualFile.parent ?: return "", stdRoot, '/')
            if (!relativeParent.isNullOrBlank()) {
                return relativeParent.replace('/', '.')
            }
        }

        val projectRoot = project.guessProjectDir()
        if (projectRoot != null && VfsUtilCore.isAncestor(projectRoot, virtualFile, false)) {
            val relativeParent = VfsUtilCore.getRelativePath(virtualFile.parent ?: return "", projectRoot, '/')
            if (relativeParent.isNullOrBlank()) {
                return projectRoot.name
            }
            return relativeParent.replace('/', '.')
        }

        return virtualFile.parent?.name ?: ""
    }
}
