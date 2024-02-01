local Packages = script.Parent.Parent.Packages
local Log = require(Packages.Log)

local clock = os.clock

local Timer = {
	_entries = {},
}

function Timer.start(label)
	local start = clock()
	if not label then
		Log.error("Timer.start: label is required")
		return
	end

	table.insert(Timer._entries, { label, start })
end

function Timer.stop()
	local stop = clock()

	local entry = table.remove(Timer._entries)
	if not entry then
		Log.error("Timer.stop: no label to stop")
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
	Log.info(string.format("%s took %.2f ms", label, duration * 1000))
end

return Timer
