local Packages = script.Parent.Parent.Packages
local Http = require(Packages.Http)
local Promise = require(Packages.Promise)

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

	if revision ~= 0 then
		return revision
	end

	local aPrerelease = if a[4] == "" then nil else a[4]
	local bPrerelease = if b[4] == "" then nil else b[4]

	-- If neither are prerelease, they are the same
	if aPrerelease == nil and bPrerelease == nil then
		return 0
	end

	-- If one is prerelease it is older
	if aPrerelease ~= nil and bPrerelease == nil then
		return -1
	end
	if aPrerelease == nil and bPrerelease ~= nil then
		return 1
	end

	-- If they are both prereleases, compare those based on number
	local aPrereleaseNumeric = string.match(aPrerelease, "(%d+).*$")
	local bPrereleaseNumeric = string.match(bPrerelease, "(%d+).*$")

	if aPrereleaseNumeric == nil or bPrereleaseNumeric == nil then
		-- If one or both lack a number, comparing isn't meaningful
		return 0
	end
	return compare(tonumber(aPrereleaseNumeric) or 0, tonumber(bPrereleaseNumeric) or 0)
end

function Version.parse(versionString: string)
	local version = { string.match(versionString, "^v?(%d+)%.(%d+)%.(%d+)(.*)$") }
	for i, v in version do
		version[i] = tonumber(v) or v
	end

	if version[4] == "" then
		version[4] = nil
	end

	return version
end

function Version.display(version)
	local output = ("%d.%d.%d"):format(version[1], version[2], version[3])

	if version[4] ~= nil then
		output = output .. version[4]
	end

	return output
end

function Version.retrieveLatestCompatible(options: {
	version: { number },
	includePrereleases: boolean?,
}): {
	version: { number },
	prerelease: boolean,
	publishedUnixTimestamp: number,
}?
	local success, releases = Http.get("https://api.github.com/repos/rojo-rbx/rojo/releases?per_page=10")
		:andThen(function(response)
			if response.code >= 400 then
				local message = string.format("HTTP %s:\n%s", tostring(response.code), response.body)

				return Promise.reject(message)
			end

			return response
		end)
		:andThen(Http.Response.json)
		:await()

	if success == false or type(releases) ~= "table" or next(releases) ~= 1 then
		return nil
	end

	-- Iterate through releases, looking for the latest compatible version
	local latestCompatible = nil
	for _, release in releases do
		-- Skip prereleases if they are not requested
		if (not options.includePrereleases) and release.prerelease then
			continue
		end

		local releaseVersion = Version.parse(release.tag_name)

		-- Skip releases that are potentially incompatible
		if releaseVersion[1] > options.version[1] then
			continue
		end

		-- Skip releases that are older than the latest compatible version
		if latestCompatible ~= nil and Version.compare(releaseVersion, latestCompatible.version) <= 0 then
			continue
		end

		latestCompatible = {
			version = releaseVersion,
			prerelease = release.prerelease,
			publishedUnixTimestamp = DateTime.fromIsoDate(release.published_at).UnixTimestamp,
		}
	end

	-- Don't return anything if the latest found is not newer than the current version
	if latestCompatible == nil or Version.compare(latestCompatible.version, options.version) <= 0 then
		return nil
	end

	return latestCompatible
end

return Version
