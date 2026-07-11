package com.example

class AlphaHandler {
    fun process(): String {
        return "alpha"
    }
}

class BetaHandler {
    fun process(): String {
        return "beta"
    }
}

class Dispatcher {
    fun run(useAlpha: Boolean): String {
        val handler: Any = if (useAlpha) AlphaHandler() else BetaHandler()
        return dispatch(handler)
    }

    private fun dispatch(handler: Any): String {
        return when (handler) {
            is AlphaHandler -> handler.process()
            is BetaHandler -> handler.process()
            else -> "unknown"
        }
    }
}
