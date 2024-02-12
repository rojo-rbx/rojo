local Settings = require(script.Parent.Settings)

local clock = os.clock

local Timer = {
	_entries = {},
}

function Timer._start(label)
	local start = clock()
	if not label then
		error("[Rojo-Timer] Timer.start: label is required", 2)
		return
	end

	table.insert(Timer._entries, { label, start })
end

function Timer._stop()
	local stop = clock()

	local entry = table.remove(Timer._entries)
	if not entry then
		error("[Rojo-Timer] Timer.stop: no label to stop", 2)
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

-- Replace functions with no-op if not in debug mode
local function no_op() end
local function setFunctions(enabled)
	if enabled then
		Timer.start = Timer._start
		Timer.stop = Timer._stop
	else
		Timer.start = no_op
		Timer.stop = no_op
	end
end

Settings:onChanged("timingLogsEnabled", setFunctions)
setFunctions(Settings:get("timingLogsEnabled"))

return Timer
