import com.android.build.gradle.LibraryExtension
import com.vanniktech.maven.publish.AndroidSingleVariantLibrary
import com.vanniktech.maven.publish.MavenPublishBaseExtension
import com.vanniktech.maven.publish.MavenPublishPlugin
import com.vanniktech.maven.publish.SonatypeHost
import java.util.Base64

val publicationVariant = "release"
val isSnapshot = project.property("IS_SNAPSHOT").toString().toBoolean()
val myVersion = project.property("LIBRARY_VERSION").toString()

// ZChat's NU7 build is published under its own group (gradle.properties ZCASH_PUBLISH_GROUP), never the official
// cash.z.ecc.android one. Dependencies that are not built here (kotlin-bip39) keep their own coordinates.
val myGroup = project.property("ZCASH_PUBLISH_GROUP").toString()
project.group = myGroup
val githubRepo = project.property("ZCASH_PUBLISH_GITHUB_REPO").toString()
val repoUrl = "https://github.com/$githubRepo"

// The javadoc jar is published unless -PZCASH_PUBLISH_JAVADOC_JAR=false is given. A build whose files must be
// reproducible (nu7-zchat/build_aar.sh) leaves it out: Dokka writes the sections of deprecated.html in an order
// that changes from run to run, and the module metadata records the hashes of that jar.
val isJavadocJarPublished = project.findProperty("ZCASH_PUBLISH_JAVADOC_JAR")?.toString()?.toBoolean() ?: true

plugins.apply(MavenPublishPlugin::class.java)
extensions.getByType<MavenPublishBaseExtension>().apply {
    configure(
        AndroidSingleVariantLibrary(
            variant = "release",
            sourcesJar = true,
            publishJavadocJar = isJavadocJarPublished
        )
    )
    publishToMavenCentral(SonatypeHost.CENTRAL_PORTAL)

    coordinates(
        groupId = myGroup,
        version = if (isSnapshot) "$myVersion-SNAPSHOT" else myVersion,
    )

    pom {
        name.set("Zcash Android Wallet SDK, ZChat NU7 build")
        description.set(
            "ZChat's build of the Zcash Android Wallet SDK 3.0.0 with NU7 support (librustzcash NU7 crates). " +
                "Not an official Electric Coin Co. or Zcash Open Development Lab release."
        )
        url.set("$repoUrl/")
        inceptionYear.set("2018")
        scm {
            url.set("$repoUrl/")
            connection.set("scm:git:git://github.com/$githubRepo.git")
            developerConnection.set("scm:git:ssh://git@github.com/$githubRepo.git")
        }
        developers {
            developer {
                id.set("zcash")
                name.set("Zcash Open Development Lab")
                url.set("https://github.com/zcash/")
            }
            developer {
                id.set("zchat")
                name.set("ZChat")
                url.set("https://github.com/${githubRepo.substringBefore('/')}/")
            }
        }
        licenses {
            license {
                name.set("MIT License")
                url.set("$repoUrl/blob/main/LICENSE")
                distribution.set("repo")
            }
        }
    }

    // signAllPublications()
}

plugins.apply("org.gradle.signing")
plugins.withId("org.gradle.signing") {
    project.the<SigningExtension>().apply {
        // Maven Central allows signing for both snapshot and release SDK versions
        isRequired = true

        val signingKey = run {
            val base64EncodedKey = project.property("ZCASH_ASCII_GPG_KEY").toString()
            if (base64EncodedKey.isNotEmpty()) {
                val keyBytes = Base64.getDecoder().decode(base64EncodedKey)
                String(keyBytes)
            } else {
                ""
            }
        }

        val signingKeyPassword = project.findProperty("ZCASH_ASCII_GPG_KEY_PASSWORD")?.toString() ?: ""

        // Without a key (a local, unsigned build such as nu7-zchat/build_aar.sh) nothing is signed; with one,
        // every publication is, as before.
        if (signingKey.isNotEmpty()) {
            useInMemoryPgpKeys(signingKey, signingKeyPassword)

            project.mavenPublications(
                object: Action<MavenPublication>{
                    override fun execute(publication: MavenPublication) {
                        project.gradleSigning.sign(publication)
                    }
                }
            )
        } else {
            // Maven Central refuses unsigned files; stop before anything is uploaded rather than after.
            project.gradle.taskGraph.whenReady {
                if (allTasks.any { it.name.contains("MavenCentral") }) {
                    throw GradleException("Publishing to Maven Central needs ZCASH_ASCII_GPG_KEY (unsigned build)")
                }
            }
        }
    }
}

private fun Project.mavenPublications(action: Action<MavenPublication>) {
    gradlePublishing.publications.withType(MavenPublication::class.java).configureEach(action)
}

private inline val Project.gradlePublishing: PublishingExtension
    get() = extensions.getByType(PublishingExtension::class.java)

private inline val Project.gradleSigning: SigningExtension
    get() = extensions.getByType(SigningExtension::class.java)
