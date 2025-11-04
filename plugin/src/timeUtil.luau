local AGE_UNITS = table.freeze({
	year = 31556909,
	month = 2629743,
	week = 604800,
	day = 86400,
	hour = 3600,
	minute = 60,
}) :: { [string]: number }

local timeUtil = {
	AGE_UNITS = AGE_UNITS,
}

function timeUtil.elapsedToText(elapsed: number): string
	if elapsed < 3 then
		return "just now"
	end

	for unit_name, unit_seconds in AGE_UNITS do
		if elapsed < unit_seconds then
			continue
		end
		local c = elapsed // unit_seconds

		--stylua: ignore
		return string.format(
			"%d %s%s ago",
			c,
			unit_name,
			if c > 1 then "s" else ""
		)
	end

	return string.format("%d seconds ago", elapsed)
end

return timeUtil
