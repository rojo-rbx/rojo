local Config = require(script.Parent.Config)

local Environment = {
	User = "User",
	Dev = "Dev",
	Test = "Test",
}

local DEFAULT_ENVIRONMENT = Config.isDevBuild and Environment.Dev or Environment.User

local VALUES = {
	LogLevel = {
		type = "IntValue",
		values = {
			[Environment.User] = 2,
			[Environment.Dev] = 4,
			[Environment.Test] = 4,
		},
	},
	TypecheckingEnabled = {
		type = "BoolValue",
		values = {
			[Environment.User] = false,
			[Environment.Dev] = true,
			[Environment.Test] = true,
		},
	},
}

local CONTAINER_NAME = "RojoDevSettings" .. Config.codename

local function getValueContainer()
	return game:FindFirstChild(CONTAINER_NAME)
end

local valueContainer = getValueContainer()

game.ChildAdded:Connect(function(child)
	local success, name = pcall(function()
		return child.Name
	end)

	if success and name == CONTAINER_NAME then
		valueContainer = child
	end
end)

local function getStoredValue(name)
	if valueContainer == nil then
		return nil
	end

	local valueObject = valueContainer:FindFirstChild(name)

	if valueObject == nil then
		return nil
	end

	return valueObject.Value
end

local function setStoredValue(name, kind, value)
	local object = valueContainer:FindFirstChild(name)

	if object == nil then
		object = Instance.new(kind)
		object.Name = name
		object.Parent = valueContainer
	end

	object.Value = value
end

local function createAllValues(environment)
	assert(Environment[environment] ~= nil, "Invalid environment")

	valueContainer = getValueContainer()

	if valueContainer == nil then
		valueContainer = Instance.new("Folder")
		valueContainer.Name = CONTAINER_NAME
		valueContainer.Parent = game
	end

	for name, value in pairs(VALUES) do
		setStoredValue(name, value.type, value.values[environment])
	end
end

local function getValue(name)
	assert(VALUES[name] ~= nil, "Invalid DevSettings name")

	local stored = getStoredValue(name)

	if stored ~= nil then
		return stored
	end

	return VALUES[name].values[DEFAULT_ENVIRONMENT]
end

local DevSettings = {}

function DevSettings:createDevSettings()
	createAllValues(Environment.Dev)
end

function DevSettings:createTestSettings()
	createAllValues(Environment.Test)
end

function DevSettings:hasChangedValues()
	return valueContainer ~= nil
end

function DevSettings:resetValues()
	if valueContainer then
		valueContainer:Destroy()
		valueContainer = nil
	end
end

function DevSettings:isEnabled()
	return valueContainer ~= nil
end

function DevSettings:getLogLevel()
	return getValue("LogLevel")
end

function DevSettings:shouldTypecheck()
	return getValue("TypecheckingEnabled")
end

function _G.ROJO_DEV_CREATE()
	DevSettings:createDevSettings()
end

return DevSettings