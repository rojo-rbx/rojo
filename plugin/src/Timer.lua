local Settings = require(script.Parent.Settings)

-- Cache enabled to avoid overhead impacting timings
local timerEnabled = Settings:get("timingLogsEnabled")
Settings:onChanged("timingLogsEnabled", function(enabled)
	timerEnabled = enabled
end)

local clock = os.clock

local Timer = {
	_entries = {},
}

function Timer.start(label)
	if not timerEnabled then
		return
	end

	local start = clock()
	if not label then
		error("[Rojo-Timer] Timer.start: label is required")
		return
	end

	table.insert(Timer._entries, { label, start })
end

function Timer.stop()
	if not timerEnabled then
		return
	end

	local stop = clock()

	local entry = table.remove(Timer._entries)
	if not entry then
		error("[Rojo-Timer] Timer.stop: no label to stop")
		return
	end

	local label = entry[1]
	if #Timer._entries > 0 then
		local priorLabels = {}
		for _, priorEntry in ipairs(Timer._entries) do
			table.insert(priorLabels, priorEntry[1])
		end
		label = table.concat(priorLabels, "/") .. "/" .. label
	end

	local start = entry[2]
	local duration = stop - start
	print(string.format("[Rojo-Timer] %s took %.3f ms", label, duration * 1000))
end

return Timer
