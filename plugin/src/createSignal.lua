--[[
	Create a new signal that can be connected to, disconnected from, and fired.

	Usage:

		local signal = createSignal()
		local disconnect = signal:connect(function(...)
			print("fired:", ...)
		end)

		signal:fire("a", "b", "c")
		disconnect()

	Avoids mutating listeners list directly to prevent iterator invalidation if
	a listener is disconnected while the signal is firing.
]]
local function createSignal()
	local listeners = {}

	local function disconnect(listener)
		local nextListeners = table.clone(listeners)
		nextListeners[listener] = nil
		listeners = nextListeners
	end

	local function connect(listener)
		local nextListeners = table.clone(listeners)
		nextListeners[listener] = true
		listeners = nextListeners

		return function()
			return disconnect(listener)
		end
	end

	local function fire(...)
		for listener in pairs(listeners) do
			listener(...)
		end
	end

	return {
		connect = connect,
		fire = fire,
	}
end

return createSignal
