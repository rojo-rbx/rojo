-- This Script is Part of the Prometheus Obfuscator by Levno_710
--
-- step.lua
--
-- This file Provides the base class for Obfuscation Steps

local logger = require("logger");
local util = require("prometheus.util");

local lookupify = util.lookupify;

local Step = {};

Step.SettingsDescriptor = {}

function Step:new(settings)
	local instance = {};
	setmetatable(instance, self);
	self.__index = self;
	
	if type(settings) ~= "table" then
		settings = {};
	end
	
	for key, data in pairs(self.SettingsDescriptor) do
		if settings[key] == nil then
			if data.default == nil then
				logger:error(string.format("The Setting \"%s\" was not provided for the Step \"%s\"", key, self.Name));
			end
			instance[key] = data.default;
		elseif(data.type == "enum") then
			local lookup = lookupify(data.values);
			if not lookup[settings[key]] then
				logger:error(string.format("Invalid value for the Setting \"%s\" of the Step \"%s\". It must be one of the following: %s", key, self.Name, table.concat(data, ", ")));
			end
			instance[key] = settings[key];
		elseif(type(settings[key]) ~= data.type) then
			logger:error(string.format("Invalid value for the Setting \"%s\" of the Step \"%s\". It must be a %s", key, self.Name, data.type));
		else
			if data.min then
				if  settings[key] < data.min then
					logger:error(string.format("Invalid value for the Setting \"%s\" of the Step \"%s\". It must be at least %d", key, self.Name, data.min));
				end
			end
			
			if data.max then
				if  settings[key] > data.max then
					logger:error(string.format("Invalid value for the Setting \"%s\" of the Step \"%s\". The biggest allowed value is %d", key, self.Name, data.min));
				end
			end
			
			instance[key] = settings[key];
		end
	end
	
	instance:init();

	return instance;
end

function Step:init()
	logger:error("Abstract Steps cannot be Created");
end

function Step:extend()
	local ext = {};
	setmetatable(ext, self);
	self.__index = self;
	return ext;
end

function Step:apply(ast, pipeline)
	logger:error("Abstract Steps cannot be Applied")
end

Step.Name = "Abstract Step";
Step.Description = "Abstract Step";

return Step;
