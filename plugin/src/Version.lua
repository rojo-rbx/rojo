local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Http = require(Packages.Http)
local Promise = require(Packages.Promise)
local Log = require(Packages.Log)

local Config = require(Plugin.Config)
local Settings = require(Plugin.Settings)
local timeUtil = require(Plugin.timeUtil)

type LatestReleaseInfo = {
	version: { number },
	prerelease: boolean,
	publishedUnixTimestamp: number,
}

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

--[[
	The GitHub API rate limit for unauthenticated requests is rather low,
	and we don't release often enough to warrant checking it more than once a day.
--]]
Version._cachedLatestCompatible = nil :: {
	value: LatestReleaseInfo?,
	timestamp: number,
}?

function Version.retrieveLatestCompatible(options: {
	version: { number },
	includePrereleases: boolean?,
}): LatestReleaseInfo?
	if Version._cachedLatestCompatible and os.clock() - Version._cachedLatestCompatible.timestamp < 60 * 60 * 24 then
		Log.debug("Using cached latest compatible version")
		return Version._cachedLatestCompatible.value
	end

	Log.debug("Retrieving latest compatible version from GitHub")

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
	local latestCompatible: LatestReleaseInfo? = nil
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
		-- Cache as nil so we don't try again for a day
		Version._cachedLatestCompatible = {
			value = nil,
			timestamp = os.clock(),
		}

		return nil
	end

	-- Cache the latest compatible version
	Version._cachedLatestCompatible = {
		value = latestCompatible,
		timestamp = os.clock(),
	}

	return latestCompatible
end

function Version.getUpdateMessage(): string?
	if not Settings:get("checkForUpdates") then
		return
	end

	local isLocalInstall = string.find(debug.traceback(), "\n[^\n]-user_.-$") ~= nil
	local latestCompatibleVersion = Version.retrieveLatestCompatible({
		version = Config.version,
		includePrereleases = isLocalInstall and Settings:get("checkForPrereleases"),
	})
	if not latestCompatibleVersion then
		return
	end

	return string.format(
		"A newer compatible version of Rojo, %s, was published %s! Go to the Rojo releases page to learn more.",
		Version.display(latestCompatibleVersion.version),
		timeUtil.elapsedToText(DateTime.now().UnixTimestamp - latestCompatibleVersion.publishedUnixTimestamp)
	)
end

return Version
