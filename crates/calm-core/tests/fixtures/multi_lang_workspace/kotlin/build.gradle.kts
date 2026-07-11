plugins {
    kotlin("jvm") version "1.9.22"
    application
}
repositories { mavenCentral() }
application { mainClass.set("com.example.MainKt") }
