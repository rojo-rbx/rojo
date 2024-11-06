local timeUtil = {}

timeUtil.AGE_UNITS = table.freeze({
	{ 31556909, "year" },
	{ 2629743, "month" },
	{ 604800, "week" },
	{ 86400, "day" },
	{ 3600, "hour" },
	{ 60, "minute" },
})

function timeUtil.elapsedToText(elapsed: number): string
	if elapsed < 3 then
		return "just now"
	end

	local ageText = string.format("%d seconds ago", elapsed)

	for _, UnitData in timeUtil.AGE_UNITS do
		local UnitSeconds, UnitName = UnitData[1], UnitData[2]
		if elapsed > UnitSeconds then
			local c = math.floor(elapsed / UnitSeconds)
			ageText = string.format("%d %s%s ago", c, UnitName, c > 1 and "s" or "")
			break
		end
	end

	return ageText
end

return timeUtil
