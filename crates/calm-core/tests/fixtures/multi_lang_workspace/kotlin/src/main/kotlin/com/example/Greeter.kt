package com.example

class Greeter(private val name: String) {
    fun greet(): String {
        return buildMessage()
    }

    private fun buildMessage(): String {
        return "Hello, $name!"
    }
}
