--!native
--!strict
local msgpack = {}

local band = bit32.band
local bor = bit32.bor
local bufferCreate = buffer.create
local bufferLen = buffer.len
local bufferCopy = buffer.copy
local readstring = buffer.readstring
local writestring = buffer.writestring
local readu8 = buffer.readu8
local readi8 = buffer.readi8
local writeu8 = buffer.writeu8
local writei8 = buffer.writei8
local lshift = bit32.lshift
local extract = bit32.extract
local ldexp = math.ldexp
local frexp = math.frexp
local floor = math.floor
local modf = math.modf
local sign = math.sign
local ssub = string.sub
local char = string.char
local sbyte = string.byte
local concat = table.concat
local tableCreate = table.create

-- MsgPack numbers are big-endian, buffer methods are little-endian
-- Will need to reverse 16 and 32 bit ints, floats and doubles
local function reverse(b: buffer, offset: number, count: number): ()
  for i=1,count//2 do
    local byte = readu8(b, offset + i - 1)
    bufferCopy(b, offset + i - 1, b, offset + count - i, 1)
    writeu8(b, offset + count - i, byte)
  end
end

local function writeu16(b: buffer, offset: number, value: number): ()
  buffer.writeu16(b, offset, value)
  reverse(b, offset, 2)
end

local function writei16(b: buffer, offset: number, value: number): ()
  buffer.writei16(b, offset, value)
  reverse(b, offset, 2)
end

local function writeu32(b: buffer, offset: number, value: number): ()
  buffer.writeu32(b, offset, value)
  reverse(b, offset, 4)
end

local function writei32(b: buffer, offset: number, value: number): ()
  buffer.writei32(b, offset, value)
  reverse(b, offset, 4)
end

local function writef32(b: buffer, offset: number, value: number): ()
  buffer.writef32(b, offset, value)
  reverse(b, offset, 4)
end

local function writef64(b: buffer, offset: number, value: number): ()
  buffer.writef64(b, offset, value)
  reverse(b, offset, 8)
end

local function readu16(b: buffer, offset: number): number
  reverse(b, offset, 2)
  return buffer.readu16(b, offset)
end

local function readi16(b: buffer, offset: number): number
  reverse(b, offset, 2)
  return buffer.readi16(b, offset)
end

local function readu32(b: buffer, offset: number): number
  reverse(b, offset, 4)
  return buffer.readu32(b, offset)
end

local function readi32(b: buffer, offset: number): number
  reverse(b, offset, 4)
  return buffer.readi32(b, offset)
end

local function readf32(b: buffer, offset: number): number
  reverse(b, offset, 4)
  return buffer.readf32(b, offset)
end

local function readf64(b: buffer, offset: number): number
  reverse(b, offset, 8)
  return buffer.readf64(b, offset)
end

local function parse(message: buffer, offset: number): (any, number)
  local byte = readu8(message, offset)

  if byte == 0xC0 then     -- nil
    return nil, offset + 1

  elseif byte == 0xC2 then -- false
    return false, offset + 1

  elseif byte == 0xC3 then -- true
    return true, offset + 1

  elseif byte == 0xC4 then -- bin 8
    local length = readu8(message, offset + 1)
    local newBuf = bufferCreate(length)
    bufferCopy(newBuf, 0, message, offset + 2, length)
    return newBuf, offset + 2 + length

  elseif byte == 0xC5 then -- bin 16
    local length = readu16(message, offset + 1)
    local newBuf = bufferCreate(length)
    bufferCopy(newBuf, 0, message, offset + 3, length)
    return newBuf, offset + 3 + length

  elseif byte == 0xC6 then -- bin 32
    local length = readu32(message, offset + 1)
    local newBuf = bufferCreate(length)
    bufferCopy(newBuf, 0, message, offset + 5, length)
    return newBuf, offset + 5 + length

  elseif byte == 0xC7 then -- ext 8
    local length = readu8(message, offset + 1)
    local newBuf = bufferCreate(length)
    bufferCopy(newBuf, 0, message, offset + 3, length)
    return msgpack.Extension.new(
             readu8(message, offset + 2),
             newBuf
           ),
           offset + 3 + length

  elseif byte == 0xC8 then -- ext 16
    local length = readu16(message, offset + 1)
    local newBuf = bufferCreate(length)
    bufferCopy(newBuf, 0, message, offset + 4, length)
    return msgpack.Extension.new(
             readu8(message, offset + 3),
             newBuf
           ),
           offset + 4 + length

  elseif byte == 0xC9 then -- ext 32
    local length = readu32(message, offset + 1)
    local newBuf = bufferCreate(length)
    bufferCopy(newBuf, 0, message, offset + 6, length)
    return msgpack.Extension.new(
             readu8(message, offset + 5),
             newBuf
           ),
           offset + 6 + length

  elseif byte == 0xCA then -- float 32
    return readf32(message, offset + 1),
           offset + 5

  elseif byte == 0xCB then -- float 64
    return readf64(message, offset + 1),
           offset + 9

  elseif byte == 0xCC then -- uint 8
    return readu8(message, offset + 1),
           offset + 2

  elseif byte == 0xCD then -- uint 16
    return readu16(message, offset + 1),
           offset + 3

  elseif byte == 0xCE then -- uint 32
    return readu32(message, offset + 1),
           offset + 5

  elseif byte == 0xCF then -- uint 64
    return msgpack.UInt64.new(
             readu32(message, offset + 1),
             readu32(message, offset + 5)
           ),
           offset + 9

  elseif byte == 0xD0 then -- int 8
    return readi8(message, offset + 1), offset + 2

  elseif byte == 0xD1 then -- int 16
    return readi16(message, offset + 1), offset + 3

  elseif byte == 0xD2 then -- int 32
    return readi32(message, offset + 1), offset + 5

  elseif byte == 0xD3 then -- int 64
    return msgpack.Int64.new(
             readu32(message, offset + 1),
             readu32(message, offset + 5)
           ),
           offset + 9

  elseif byte == 0xD4 then -- fixext 1
    local newBuf = bufferCreate(1)
    bufferCopy(newBuf, 0, message, offset + 2, 1)
    return msgpack.Extension.new(
             readu8(message, offset + 1),
             newBuf
           ),
           offset + 3

  elseif byte == 0xD5 then -- fixext 2
    local newBuf = bufferCreate(2)
    bufferCopy(newBuf, 0, message, offset + 2, 2)
    return msgpack.Extension.new(
             readu8(message, offset + 1),
             newBuf
           ),
           offset + 4

  elseif byte == 0xD6 then -- fixext 4
    local newBuf = bufferCreate(4)
    bufferCopy(newBuf, 0, message, offset + 2, 4)
    return msgpack.Extension.new(
             readu8(message, offset + 1),
             newBuf
           ),
           offset + 6

  elseif byte == 0xD7 then -- fixext 8
    local newBuf = bufferCreate(8)
    bufferCopy(newBuf, 0, message, offset + 2, 8)
    return msgpack.Extension.new(
             readu8(message, offset + 1),
             newBuf
           ),
           offset + 10

  elseif byte == 0xD8 then -- fixext 16
    local newBuf = bufferCreate(16)
    bufferCopy(newBuf, 0, message, offset + 2, 16)
    return msgpack.Extension.new(
             readu8(message, offset + 1),
             newBuf
           ),
           offset + 18

  elseif byte == 0xD9 then -- str 8
    local length = readu8(message, offset + 1)
    return readstring(message, offset + 2, length),
           offset + 2 + length

  elseif byte == 0xDA then -- str 16
    local length = readu16(message, offset + 1)
    return readstring(message, offset + 3, length),
           offset + 3 + length

  elseif byte == 0xDB then -- str 32
    local length = readu32(message, offset + 1)
    return readstring(message, offset + 5, length),
           offset + 5 + length

  elseif byte == 0xDC then -- array 16
    local length = readu16(message, offset + 1)
    local array = tableCreate(length)
    local newOffset = offset + 3

    for i=1,length do
      array[i], newOffset = parse(message, newOffset)
    end

    return array, newOffset

  elseif byte == 0xDD then -- array 32
    local length = readu32(message, offset + 1)
    local array = tableCreate(length)
    local newOffset = offset + 5

    for i=1,length do
      array[i], newOffset = parse(message, newOffset)
    end

    return array, newOffset

  elseif byte == 0xDE then -- map 16
    local length = readu16(message, offset + 1)
    local dictionary = {}
    local newOffset = offset + 3
    local key

    for _=1,length do
      key, newOffset = parse(message, newOffset)
      dictionary[key], newOffset = parse(message, newOffset)
    end

    return dictionary, newOffset

  elseif byte == 0xDF then -- map 32
    local length = readu32(message, offset + 1)
    local dictionary = {}
    local newOffset = offset + 5
    local key

    for _=1,length do
      key, newOffset = parse(message, newOffset)
      dictionary[key], newOffset = parse(message, newOffset)
    end

    return dictionary, newOffset

  elseif byte >= 0xE0 then -- negative fixint
    return byte - 256, offset + 1

  elseif byte <= 0x7F then -- positive fixint
    return byte, offset + 1

  elseif byte - 0x80 <= 0x8F - 0x80 then -- fixmap
    local length = band(byte, 0xF)
    local dictionary = {}
    local newOffset = offset + 1
    local key

    for _=1,length do
      key, newOffset = parse(message, newOffset)
      dictionary[key], newOffset = parse(message, newOffset)
    end

    return dictionary, newOffset

  elseif byte - 0x90 <= 0x9F - 0x90 then -- fixarray
    local length = band(byte, 0xF)
    local array = tableCreate(length)
    local newOffset = offset + 1

    for i=1,length do
      array[i], newOffset = parse(message, newOffset)
    end

    return array, newOffset

  elseif byte - 0xA0 <= 0xBF - 0xA0 then -- fixstr
    local length = byte - 0xA0
    return readstring(message, offset + 1, length),
           offset + 1 + length

  end

  error("Not all decoder cases are handled, report as bug to msgpack-luau maintainer")
end

local function computeLength(data: any, tableSet: {[any]: boolean}): number
  local dtype = type(data)
  if data == nil then
    return 1
  elseif dtype == "boolean" then
    return 1
  elseif dtype == "string" then
    local length = #data

    if length <= 31 then
      return 1 + length
    elseif length <= 0xFF then
      return 2 + length
    elseif length <= 0xFFFF then
      return 3 + length
    elseif length <= 0xFFFFFFFF then
      return 5 + length
    end

    error("Could not encode - too long string")

  elseif dtype == "buffer" then
    local length = bufferLen(data)

    if length <= 0xFF then
      return 2 + length
    elseif length <= 0xFFFF then
      return 3 + length
    elseif length <= 0xFFFFFFFF then
      return 5 + length
    end

    error("Could not encode - too long binary buffer")

  elseif dtype == "number" then
    -- represents NaN, Inf, -Inf as float 32 to save space
    if data == 0 then
      return 1
    elseif data ~= data then -- NaN
      return 5
    elseif data == math.huge then
      return 5
    elseif data == -math.huge then
      return 5
    end

    local integral, fractional = modf(data)
    local sign = sign(data)

    if fractional ~= 0 or integral > 0xFFFFFFFF or integral < -0x80000000 then
      -- float 64
      return 9
    end

    if sign > 0 then
      if integral <= 127 then -- positive fixint
        return 1
      elseif integral <= 0xFF then -- uint 8
        return 2
      elseif integral <= 0xFFFF then -- uint 16
        return 3
      elseif integral <= 0xFFFFFFFF then -- uint 32
        return 5
      end
    else
      if integral >= -0x20 then -- negative fixint
        return 1
      elseif integral >= -0x80 then -- int 8
        return 2
      elseif integral >= -0x8000 then -- int 16
        return 3
      elseif integral >= -0x80000000 then -- int 32
        return 5
      end
    end

    error(string.format("Could not encode - unhandled number \"%s\"", typeof(data)))

  elseif dtype == "table" then
    local msgpackType = data._msgpackType

    if msgpackType then
      if msgpackType == msgpack.Int64 or msgpackType == msgpack.UInt64 then
        return 9
      elseif msgpackType == msgpack.Extension then
        local length = bufferLen(data.data)

        if length == 1 then
          return 3
        elseif length == 2 then
          return 4
        elseif length == 4 then
          return 6
        elseif length == 8 then
          return 10
        elseif length == 16 then
          return 18
        elseif length <= 0xFF then
          return 3 + length
        elseif length <= 0xFFFF then
          return 4 + length
        elseif length <= 0xFFFFFFFF then
          return 6 + length
        end

        error("Could not encode - too long extension data")
      end
    end

    if tableSet[data] then
      error("Can not serialize cyclic table")
    else
      tableSet[data] = true
    end

    local length = #data
    local mapLength = 0

    for _,_ in pairs(data) do
      mapLength += 1
    end

    local headerLen
    if mapLength <= 15 then
      headerLen = 1
    elseif mapLength <= 0xFFFF then
      headerLen = 3
    elseif mapLength <= 0xFFFFFFFF then
      headerLen = 5
    else
      if length == mapLength then
        error("Could not encode - too long array")
      else
        error("Could not encode - too long map")
      end
    end

    if length == mapLength then -- array
      local contentLen = 0
      for _,v in ipairs(data) do
        contentLen += computeLength(v, tableSet)
      end

      return headerLen + contentLen

    else -- map
      local contentLen = 0
      for k,v in pairs(data) do
        contentLen += computeLength(k, tableSet)
        contentLen += computeLength(v, tableSet)
      end

      return headerLen + contentLen
    end
  end

  error(string.format("Could not encode - unsupported datatype \"%s\"", typeof(data)))
end

local extensionTypeLUT = {
  [1] = 0xD4,
  [2] = 0xD5,
  [4] = 0xD6,
  [8] = 0xD7,
  [16] = 0xD8,
}

local function encode(result: buffer, offset: number, data: any): number

  local dtype = type(data)
  if data == nil then
    writestring(result, offset, "\xC0")
    return offset + 1
  elseif data == false then
    writestring(result, offset, "\xC2")
    return offset + 1
  elseif data == true then
    writestring(result, offset, "\xC3")
    return offset + 1
  elseif dtype == "string" then
    local length = #data

    if length <= 31 then
      writeu8(result, offset, bor(0xA0, length))
      writestring(result, offset + 1, data)
      return offset + 1 + length
    elseif length <= 0xFF then
      writeu8(result, offset, 0xD9)
      writeu8(result, offset + 1, length)
      writestring(result, offset + 2, data)
      return offset + 2 + length
    elseif length <= 0xFFFF then
      writeu8(result, offset, 0xDA)
      writeu16(result, offset + 1, length)
      writestring(result, offset + 3, data)
      return offset + 3 + length
    elseif length <= 0xFFFFFFFF then
      writeu8(result, offset, 0xDB)
      writeu32(result, offset + 1, length)
      writestring(result, offset + 5, data)
      return offset + 5 + length
    end

    error("Could not encode - too long string")

  elseif dtype == "buffer" then
    local length = bufferLen(data)

    if length <= 0xFF then
      writeu8(result, offset, 0xC4)
      writeu8(result, offset + 1, length)
      bufferCopy(result, offset + 2, data)
      return offset + 2 + length
    elseif length <= 0xFFFF then
      writeu8(result, offset, 0xC5)
      writeu16(result, offset + 1, length)
      bufferCopy(result, offset + 3, data)
      return offset + 3 + length
    elseif length <= 0xFFFFFFFF then
      writeu8(result, offset, 0xC6)
      writeu32(result, offset + 1, length)
      bufferCopy(result, offset + 5, data)
      return offset + 5 + length
    end

    error("Could not encode - too long binary buffer")

  elseif dtype == "number" then
    -- represents NaN, Inf, -Inf as float 32 to save space
    if data == 0 then
      writeu8(result, offset, 0)
      return offset + 1
    elseif data ~= data then -- NaN
      writestring(result, offset, "\xCA\x7F\x80\x00\x01")
      return offset + 5
    elseif data == math.huge then
      writestring(result, offset, "\xCA\x7F\x80\x00\x00")
      return offset + 5
    elseif data == -math.huge then
      writestring(result, offset, "\xCA\xFF\x80\x00\x00")
      return offset + 5
    end

    local integral, fractional = modf(data)
    local sign = sign(data)

    if fractional ~= 0 or integral > 0xFFFFFFFF or integral < -0x80000000 then
      -- float 64
      writeu8(result, offset, 0xCB)
      writef64(result, offset + 1, data)
      return offset + 9
    end

    if sign > 0 then
      if integral <= 127 then -- positive fixint
        writeu8(result, offset, integral)
        return offset + 1
      elseif integral <= 0xFF then -- uint 8
        writeu8(result, offset, 0xCC)
        writeu8(result, offset + 1, integral)
        return offset + 2
      elseif integral <= 0xFFFF then -- uint 16
        writeu8(result, offset, 0xCD)
        writeu16(result, offset + 1, integral)
        return offset + 3
      elseif integral <= 0xFFFFFFFF then -- uint 32
        writeu8(result, offset, 0xCE)
        writeu32(result, offset + 1, integral)
        return offset + 5
      end
    else
      if integral >= -0x20 then -- negative fixint
        writeu8(result, offset, bor(0xE0, extract(integral, 0, 5)))
        return offset + 1
      elseif integral >= -0x80 then -- int 8
        writeu8(result, offset, 0xD0)
        writei8(result, offset + 1, integral)
        return offset + 2
      elseif integral >= -0x8000 then -- int 16
        writeu8(result, offset, 0xD1)
        writei16(result, offset + 1, integral)
        return offset + 3
      elseif integral >= -0x80000000 then -- int 32
        writeu8(result, offset, 0xD2)
        writei32(result, offset + 1, integral)
        return offset + 5
      end
    end

    error(string.format("Could not encode - unhandled number \"%s\"", typeof(data)))

  elseif dtype == "table" then
    local msgpackType = data._msgpackType

    if msgpackType then
      if msgpackType == msgpack.Int64 or msgpackType == msgpack.UInt64 then
        local intType = if msgpackType == msgpack.UInt64 then 0xCF else 0xD3
        writeu8(result, offset, intType)
        writeu32(result, offset + 1, data.mostSignificantPart)
        writeu32(result, offset + 5, data.leastSignificantPart)
        return offset + 9
      elseif msgpackType == msgpack.Extension then
        local length = bufferLen(data.data)
        local extType = extensionTypeLUT[length]

        if extType then
          writeu8(result, offset, extType)
          writeu8(result, offset + 1, data.type)
          bufferCopy(result, offset + 2, data.data)
          return offset + 2 + length
        end

        if length <= 0xFF then
          writeu8(result, offset, 0xC7)
          writeu8(result, offset + 1, length)
          writeu8(result, offset + 2, data.type)
          bufferCopy(result, offset + 3, data.data)
          return offset + 3 + length
        elseif length <= 0xFFFF then
          writeu8(result, offset, 0xC8)
          writeu16(result, offset + 1, length)
          writeu8(result, offset + 3, data.type)
          bufferCopy(result, offset + 4, data.data)
          return offset + 4 + length
        elseif length <= 0xFFFFFFFF then
          writeu8(result, offset, 0xC9)
          writeu32(result, offset + 1, length)
          writeu8(result, offset + 5, data.type)
          bufferCopy(result, offset + 6, data.data)
          return offset + 6 + length
        end

        error("Could not encode - too long extension data")
      end
    end

    local length = #data
    local mapLength = 0

    for _,_ in pairs(data) do
      mapLength += 1
    end

    if length == mapLength then -- array
      local newOffset = offset
      if length <= 15 then
        writeu8(result, offset, bor(0x90, mapLength))
        newOffset += 1
      elseif length <= 0xFFFF then
        writeu8(result, offset, 0xDC)
        writeu16(result, offset + 1, length)
        newOffset += 3
      elseif length <= 0xFFFFFFFF then
        writeu8(result, offset, 0xDD)
        writeu32(result, offset + 1, length)
        newOffset += 5
      else
        error("Could not encode - too long array")
      end

      for _,v in ipairs(data) do
        newOffset = encode(result, newOffset, v)
      end

      return newOffset

    else -- map
      local newOffset = offset
      if mapLength <= 15 then
        writeu8(result, offset, bor(0x80, mapLength))
        newOffset += 1
      elseif mapLength <= 0xFFFF then
        writeu8(result, offset, 0xDE)
        writeu16(result, offset + 1, mapLength)
        newOffset += 3
      elseif mapLength <= 0xFFFFFFFF then
        writeu8(result, offset, 0xDF)
        writeu32(result, offset + 1, mapLength)
        newOffset += 5
      else
        error("Could not encode - too long map")
      end

      for k,v in pairs(data) do
        newOffset = encode(result, newOffset, k)
        newOffset = encode(result, newOffset, v)
      end

      return newOffset
    end
  end

  error(string.format("Could not encode - unsupported datatype \"%s\"", typeof(data)))
end

msgpack.Int64 = {}

function msgpack.Int64.new(mostSignificantPart: number, leastSignificantPart: number): Int64
  return {
    _msgpackType = msgpack.Int64,
    mostSignificantPart = mostSignificantPart,
    leastSignificantPart = leastSignificantPart
  }
end

msgpack.UInt64 = {}

function msgpack.UInt64.new(mostSignificantPart: number, leastSignificantPart: number): UInt64
  return {
    _msgpackType = msgpack.UInt64,
    mostSignificantPart = mostSignificantPart,
    leastSignificantPart = leastSignificantPart
  }
end

msgpack.Extension = {}

function msgpack.Extension.new(extensionType: number, blob: buffer): Extension
  return {
    _msgpackType = msgpack.Extension,
    type = extensionType,
    data = blob
  }
end

function msgpack.utf8Encode(message: string): string
  local messageLength = #message
  local nBytes = math.ceil(messageLength * (8 / 7))
  local result = bufferCreate(nBytes)

  local bitPointer = 0
  for i=1,nBytes do
    local j = 1 + floor(bitPointer / 8)
    local bitRemainder = bitPointer % 8
    local byte = sbyte(message, j)

    if bitRemainder == 0 then
      writeu8(result, i-1, extract(byte, 1, 7))
    elseif bitRemainder == 1 then
      writeu8(result, i-1, extract(byte, 0, 7))
    else
      local nextByte = sbyte(message, j+1) or 0
      writeu8(result, i-1, bor(
        lshift(extract(byte, 0, 8 - bitRemainder), bitRemainder - 1),
        extract(nextByte, 9 - bitRemainder, bitRemainder - 1)
      ))
    end

    bitPointer += 7
  end

  return buffer.tostring(result)
end

function msgpack.utf8Decode(message: string): string
  local nBytes = floor(#message *  7 / 8)
  local result = bufferCreate(nBytes)

  local bitPointer = 0
  for i=1,nBytes do
    local bitRemainder = bitPointer % 7
    local byte = sbyte(message, 1 + floor(bitPointer / 7))
    local nextByte = sbyte(message, 2 + floor(bitPointer / 7))

    writeu8(result, i-1, bor(
      lshift(extract(byte, 0, 7 - bitRemainder), bitRemainder + 1),
      extract(nextByte, 6 - bitRemainder, 1 + bitRemainder)
    ))

    bitPointer += 8
  end

  return buffer.tostring(result)
end

function msgpack.decode(message: string): any
  if message == "" then
    error("Could not decode - input string is too short")
  end
  local messageBuf = buffer.fromstring(message)
  return (parse(messageBuf, 0))
end

function msgpack.encode(data: any): string
  local length = computeLength(data, {})
  local result = bufferCreate(length)
  encode(result, 0, data)
  return buffer.tostring(result)
end

export type Int64     = { _msgpackType: typeof(msgpack.Int64), mostSignificantPart: number, leastSignificantPart: number }
export type UInt64    = { _msgpackType: typeof(msgpack.UInt64), mostSignificantPart: number, leastSignificantPart: number }
export type Extension = { _msgpackType: typeof(msgpack.Extension), type:number, data: buffer }

return msgpack

--[[
MIT License

Copyright (c) 2024 Valts Liepiņš

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
]]
