local function compare(a, b)
	if a > b then
		return 1
	elseif a < b then
		return -1
	end

	return 0
end

local Version = {}

--[[
	Compares two versions of the form {major, minor, revision}.

	If a is newer than b, 1.
	If a is older than b, -1.
	If a and b are the same, 0.
]]
function Version.compare(a, b)
	local major = compare(a[1], b[1])
	local minor = compare(a[2] or 0, b[2] or 0)
	local revision = compare(a[3] or 0, b[3] or 0)

	if major ~= 0 then
		return major
	end

	if minor ~= 0 then
		return minor
	end

	return revision
end

function Version.display(version)
	return table.concat(version, ".")
end

return Version
