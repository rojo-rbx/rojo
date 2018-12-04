local Config = require(script.Parent.Config)

local function getValueContainer()
	return game:FindFirstChild("RojoDev-" .. Config.codename)
end

local valueContainer = getValueContainer()

local function create()
	valueContainer = getValueContainer()

	if valueContainer == nil then
		valueContainer = Instance.new("Folder")
		valueContainer.Name = "RojoDev-" .. Config.codename
		valueContainer.Parent = game
	end

	local valueLogLevel = valueContainer:FindFirstChild("LogLevel")
	if valueLogLevel == nil then
		valueLogLevel = Instance.new("IntValue")
		valueLogLevel.Name = "LogLevel"
		valueLogLevel.Value = 2
		valueLogLevel.Parent = valueContainer
	end
end

_G[("ROJO_%s_DEV_CREATE"):format(Config.codename:upper())] = create

local function getValue(name)
	if valueContainer == nil then
		return nil
	end

	local valueObject = valueContainer:FindFirstChild(name)

	if valueObject == nil then
		return nil
	end

	return valueObject.Value
end

local DevSettings = {}

function DevSettings:isEnabled()
	return valueContainer ~= nil
end

function DevSettings:getLogLevel()
	return getValue("LogLevel")
end

return DevSettings