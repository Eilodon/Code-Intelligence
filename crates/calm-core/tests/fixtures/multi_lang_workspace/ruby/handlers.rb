# typed: false

class AlphaHandler
  def process
    "alpha"
  end
end

class BetaHandler
  def process
    "beta"
  end
end

class Dispatcher
  def run(use_alpha)
    handler = use_alpha ? AlphaHandler.new : BetaHandler.new
    route(handler)
  end

  def route(handler)
    case handler
    when AlphaHandler
      handler.process
    when BetaHandler
      handler.process
    else
      "unknown"
    end
  end
end
