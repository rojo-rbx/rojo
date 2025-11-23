-- This Script is Part of the Prometheus Obfuscator by Levno_710
--
-- logger.lua

local logger = {}
local config = require("config");
local colors = require("colors");

logger.LogLevel = {
	Error = 0,
	Warn = 1,
	Log = 2,
	Info = 2,
	Debug = 3,
}

logger.logLevel = logger.LogLevel.Error;

-- functions

logger.debugCallback = function(...)
	print(colors(config.NameUpper .. ": " ..  ..., "grey"));
end;

logger.logCallback = function(...)
	print(colors(config.NameUpper .. ": ", "magenta") .. ...);
end;

logger.warnCallback = function(...)
	print(colors(config.NameUpper .. ": " .. ..., "yellow"));
end;

logger.errorCallback = function(...)
	print(colors(config.NameUpper .. ": " .. ..., "red"))
	error(...);
end;

-- interface methods

function logger:debug(...)
	if self.logLevel >= self.LogLevel.Debug then
		self.debugCallback(...);
	end
end

function logger:log(...)
	if self.logLevel >= self.LogLevel.Log then
		self.logCallback(...);
	end
end

function logger:info(...)
	if self.logLevel >= self.LogLevel.Log then
		self.logCallback(...);
	end
end

function logger:warn(...)
	if self.logLevel >= self.LogLevel.Warn then
		self.warnCallback(...);
	end
end

function logger:error(...)
	self.errorCallback(...);
	error(config.NameUpper .. ": logger.errorCallback did not throw an Error!");
end


return logger;