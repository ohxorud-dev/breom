import org.jetbrains.intellij.platform.gradle.TestFrameworkType

buildscript {
    repositories {
        mavenCentral()
    }
    dependencies {
        classpath("it.unimi.dsi:fastutil:8.5.12")
    }
}

plugins {
    id("java")
    alias(libs.plugins.kotlin)
    alias(libs.plugins.intelliJPlatform)
    alias(libs.plugins.grammarkit)
}

group = providers.gradleProperty("pluginGroup").get()
version = providers.gradleProperty("pluginVersion").get()

kotlin {
    jvmToolchain(21)
}

repositories {
    mavenCentral()

    intellijPlatform {
        defaultRepositories()
    }
    maven { url = uri("https://www.jetbrains.com/intellij-repository/releases") }
}

dependencies {
    testImplementation(libs.junit)
    testImplementation(libs.opentest4j)

    intellijPlatform {
        intellijIdea(providers.gradleProperty("platformVersion"))
        bundledPlugins(providers.gradleProperty("platformBundledPlugins").map { it.split(',') })
        plugins(providers.gradleProperty("platformPlugins").map { it.split(',') })
        bundledModules(providers.gradleProperty("platformBundledModules").map { it.split(',') })
        testFramework(TestFrameworkType.Platform)
    }
}


intellijPlatform {
    pluginConfiguration {
        name = providers.gradleProperty("pluginName")
        version = providers.gradleProperty("pluginVersion")
        description = providers.provider { "Breom language support for IntelliJ-based IDEs." }

        ideaVersion {
            sinceBuild = providers.gradleProperty("pluginSinceBuild")
        }
    }

    signing {
        certificateChain = providers.environmentVariable("CERTIFICATE_CHAIN")
        privateKey = providers.environmentVariable("PRIVATE_KEY")
        password = providers.environmentVariable("PRIVATE_KEY_PASSWORD")
    }

    publishing {
        token = providers.environmentVariable("PUBLISH_TOKEN")
        channels = providers.gradleProperty("pluginVersion").map { listOf(it.substringAfter('-', "").substringBefore('.').ifEmpty { "default" }) }
    }

    pluginVerification {
        ides {
            recommended()
        }
    }
}

sourceSets {
    main {
        java.srcDirs("src/main/gen")
    }
}

tasks {
    wrapper {
        gradleVersion = providers.gradleProperty("gradleVersion").get()
    }

    generateLexer {
        sourceFile.set(file("src/main/grammars/Breom.flex"))
        targetOutputDir.set(file("src/main/gen/org/ohxorud/breom/jetbrains/lexer"))
        purgeOldFiles.set(true)
    }

    generateParser {
        sourceFile.set(file("src/main/grammars/Breom.bnf"))
        targetRootOutputDir.set(file("src/main/gen"))
        pathToParser.set("org/ohxorud/breom/jetbrains/parser/BreomParser.java")
        pathToPsiRoot.set("org/ohxorud/breom/jetbrains/psi")
        purgeOldFiles.set(true)
    }

    named("compileKotlin") {
        dependsOn(generateLexer)
        dependsOn(generateParser)
    }
}

intellijPlatformTesting {
    runIde {
        register("runIdeForUiTests") {
            task {
                jvmArgumentProviders += CommandLineArgumentProvider {
                    listOf(
                        "-Drobot-server.port=8082",
                        "-Dide.mac.message.dialogs.as.sheets=false",
                        "-Djb.privacy.policy.text=<!--999.999-->",
                        "-Djb.consents.confirmation.enabled=false",
                    )
                }
                environment("BREOM_HOME", "../..")
            }

            plugins {
                robotServerPlugin()
            }
        }
    }
}
