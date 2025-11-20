-- This Script is Part of the Prometheus Obfuscator by Levno_710
--
-- util.lua
-- This file Provides some utility functions

local logger = require("logger");
local bit32  = require("prometheus.bit").bit32;

local MAX_UNPACK_COUNT = 195;

local function lookupify(tb)
	local tb2 = {};
	for _, v in ipairs(tb) do
		tb2[v] = true
	end
	return tb2
end

local function unlookupify(tb)
	local tb2 = {};
	for v, _ in pairs(tb) do
		table.insert(tb2, v);
	end
	return tb2;
end

local function escape(str)
	return str:gsub(".", function(char)
		if char:match("[^ %-~\n\t\a\b\v\r\"\']") then -- Check if non Printable ASCII Character
			return string.format("\\%03d", string.byte(char))
		end
		if(char == "\\") then
			return "\\\\";
		end
		if(char == "\n") then
			return "\\n";
		end
		if(char == "\r") then
			return "\\r";
		end
		if(char == "\t") then
			return "\\t";
		end
		if(char == "\a") then
			return "\\a";
		end
		if(char == "\b") then
			return "\\b";
		end
		if(char == "\v") then
			return "\\v";
		end
		if(char == "\"") then
			return "\\\"";
		end
		if(char == "\'") then
			return "\\\'";
		end
		return char;
	end)
end

local function chararray(str)
	local tb = {};
	for i = 1, str:len(), 1 do
		table.insert(tb, str:sub(i, i));
	end
	return tb;
end

local function keys(tb)
	local keyset={}
	local n=0
	for k,v in pairs(tb) do
		n=n+1
		keyset[n]=k
	end
	return keyset
end

local utf8char;
do
	local string_char = string.char
	function utf8char(cp)
	  if cp < 128 then
		return string_char(cp)
	  end
	  local suffix = cp % 64
	  local c4 = 128 + suffix
	  cp = (cp - suffix) / 64
	  if cp < 32 then
		return string_char(192 + cp, c4)
	  end
	  suffix = cp % 64
	  local c3 = 128 + suffix
	  cp = (cp - suffix) / 64
	  if cp < 16 then
		return string_char(224 + cp, c3, c4)
	  end
	  suffix = cp % 64
	  cp = (cp - suffix) / 64
	  return string_char(240 + cp, 128 + suffix, c3, c4)
	end
  end

local function shuffle(tb)
	for i = #tb, 2, -1 do
		local j = math.random(i)
		tb[i], tb[j] = tb[j], tb[i]
	end
	return tb
end
local function shuffle_string(str)
    local len = #str
    local t = {}
    for i = 1, len do
        t[i] = string.sub(str, i, i)
    end
    for i = 1, len do
        local j = math.random(i, len)
        t[i], t[j] = t[j], t[i]
    end
    return table.concat(t)
end

local function readDouble(bytes) 
	local sign = 1
	local mantissa = bytes[2] % 2^4
	for i = 3, 8 do
		mantissa = mantissa * 256 + bytes[i]
	end
	if bytes[1] > 127 then sign = -1 end
	local exponent = (bytes[1] % 128) * 2^4 + math.floor(bytes[2] / 2^4)

	if exponent == 0 then
		return 0
	end
	mantissa = (math.ldexp(mantissa, -52) + 1) * sign
	return math.ldexp(mantissa, exponent - 1023)
end

local function writeDouble(num)
	local bytes = {0,0,0,0, 0,0,0,0}
	if num == 0 then
		return bytes
	end
	local anum = math.abs(num)

	local mantissa, exponent = math.frexp(anum)
	exponent = exponent - 1
	mantissa = mantissa * 2 - 1
	local sign = num ~= anum and 128 or 0
	exponent = exponent + 1023

	bytes[1] = sign + math.floor(exponent / 2^4)
	mantissa = mantissa * 2^4
	local currentmantissa = math.floor(mantissa)
	mantissa = mantissa - currentmantissa
	bytes[2] = (exponent % 2^4) * 2^4 + currentmantissa
	for i= 3, 8 do
		mantissa = mantissa * 2^8
		currentmantissa = math.floor(mantissa)
		mantissa = mantissa - currentmantissa
		bytes[i] = currentmantissa
	end
	return bytes
end

local function writeU16(u16)
	if (u16 < 0 or u16 > 65535) then
		logger:error(string.format("u16 out of bounds: %d", u16));
	end
	local lower = bit32.band(u16, 255);
	local upper = bit32.rshift(u16, 8);
	return {lower, upper}
end

local function readU16(arr)
	return bit32.bor(arr[1], bit32.lshift(arr[2], 8));
end

local function writeU24(u24)
	if(u24 < 0 or u24 > 16777215) then
		logger:error(string.format("u24 out of bounds: %d", u24));
	end
	
	local arr = {};
	for i = 0, 2 do
		arr[i + 1] = bit32.band(bit32.rshift(u24, 8 * i), 255);
	end
	return arr;
end

local function readU24(arr)
	local val = 0;

	for i = 0, 2 do
		val = bit32.bor(val, bit32.lshift(arr[i + 1], 8 * i));
	end

	return val;
end

local function writeU32(u32)
	if(u32 < 0 or u32 > 4294967295) then
		logger:error(string.format("u32 out of bounds: %d", u32));
	end

	local arr = {};
	for i = 0, 3 do
		arr[i + 1] = bit32.band(bit32.rshift(u32, 8 * i), 255);
	end
	return arr;
end

local function readU32(arr)
	local val = 0;

	for i = 0, 3 do
		val = bit32.bor(val, bit32.lshift(arr[i + 1], 8 * i));
	end

	return val;
end

local function bytesToString(arr)
	local length = arr.n or #arr;

	if length < MAX_UNPACK_COUNT then
		return string.char(table.unpack(arr))
	end

	local str = "";
	local overflow = length % MAX_UNPACK_COUNT;

	for i = 1, (#arr - overflow) / MAX_UNPACK_COUNT do
		str = str .. string.char(table.unpack(arr, (i - 1) * MAX_UNPACK_COUNT + 1, i * MAX_UNPACK_COUNT));
	end

	return str..(overflow > 0 and string.char(table.unpack(arr, length - overflow + 1, length)) or "");
end

local function isNaN(n)
	return type(n) == "number" and n ~= n;
end

local function isInt(n)
	return math.floor(n) == n;
end

local function isU32(n)
	return n >= 0 and n <= 4294967295 and isInt(n);
end

local function toBits(num)
    -- returns a table of bits, least significant first.
    local t={} -- will contain the bits
	local rest;
    while num>0 do
        rest=math.fmod(num,2)
        t[#t+1]=rest
        num=(num-rest)/2
    end
    return t
end


local function readonly(obj)
	local r = newproxy(true);
	getmetatable(r).__index = obj;
	return r;
end

return {
	lookupify = lookupify,
	unlookupify = unlookupify,
	escape = escape,
	chararray = chararray,
	keys = keys,
	shuffle = shuffle,
	shuffle_string = shuffle_string,
	readDouble = readDouble,
	writeDouble = writeDouble,
	readU16 = readU16,
	writeU16 = writeU16,
	readU32 = readU32,
	writeU32 = writeU32,
	readU24 = readU24,
	writeU24 = writeU24,
	isNaN = isNaN,
	isU32 = isU32,
	isInt = isInt,
	utf8char = utf8char,
	toBits = toBits,
	bytesToString = bytesToString,
	readonly = readonly,
}
