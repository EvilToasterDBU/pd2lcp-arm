from __future__ import annotations
import struct
from pathlib import Path
from typing import Iterable

HEADER = struct.Struct('<HHIBIII')
NODE = struct.Struct('<BHI II H'.replace(' ',''))


def read_tbl(path: str | Path) -> list[tuple[str,str]]:
    data=Path(path).read_bytes()
    if len(data) < HEADER.size:
        raise ValueError(f'{path}: short header')
    crc,count,hash_size,version,data_start,max_tries,file_size=HEADER.unpack_from(data)
    if file_size != len(data) or data_start != HEADER.size + count*2 + hash_size*NODE.size:
        raise ValueError(f'{path}: invalid header {file_size=} {len(data)=} {data_start=} {count=} {hash_size=}')
    nodes_off=HEADER.size+count*2
    entries=[]
    for i in range(hash_size):
        active,index,hash_value,key_off,val_off,val_len=NODE.unpack_from(data,nodes_off+i*NODE.size)
        if not active:
            continue
        def read_string(off:int, encoding:str)->str:
            if off < data_start or off >= len(data):
                raise ValueError(f'{path}: bad string offset {off}')
            end=data.find(b'\0',off)
            if end < 0:
                raise ValueError(f'{path}: unterminated string at {off}')
            return data[off:end].decode(encoding,errors='strict')
        key=read_string(key_off,'cp1252')
        val=read_string(val_off,'utf-8')
        entries.append((index,key,val))
    entries.sort()
    if len(entries)!=count:
        raise ValueError(f'{path}: active count {len(entries)} != {count}')
    keys=[k for _,k,_ in entries]
    return [(k,v) for _,k,v in entries]


def hash_key(key: bytes, size: int) -> int:
    h=0
    for byte in key:
        h=((h<<4)+byte)&0xffffffff
        high=h & 0xf0000000
        if high:
            h=(h & 0x0fffffff) ^ (high>>24)
    return h % size


def crc16(data: bytes)->int:
    crc=0xffff
    for byte in data:
        crc ^= byte << 8
        for _ in range(8):
            crc = ((crc << 1) ^ 0x1021) & 0xffff if crc & 0x8000 else (crc << 1) & 0xffff
    return crc


def write_tbl(path: str|Path, rows: Iterable[tuple[str,str]])->None:
    rows=list(rows)
    if not rows or len(rows)>65535:
        raise ValueError('invalid row count')
    seen=set()
    normalized=[]
    for key,val in rows:
        if key in seen: raise ValueError(f'duplicate key {key}')
        seen.add(key)
        normalized.append((key.encode('cp1252'),val.encode('utf-8')))
    n=len(normalized)
    data_start=HEADER.size+n*2+n*NODE.size
    text_blob=bytearray()
    nodes=[None]*n
    indices=[]
    collision_count=0
    for i,(key,val) in enumerate(normalized):
        hv=hash_key(key,n)
        slot=hv
        collisions=0
        while nodes[slot] is not None:
            collisions+=1
            slot=(slot+1)%n
        collision_count=max(collision_count,collisions)
        key_off=data_start+len(text_blob)
        text_blob.extend(key+b'\0')
        val_off=data_start+len(text_blob)
        text_blob.extend(val+b'\0')
        nodes[slot]=(1,i,hv,key_off,val_off,len(val)+1)
        indices.append(slot)
    table=bytearray(HEADER.size)
    for i in indices: table.extend(struct.pack('<H',i))
    for node in nodes: table.extend(NODE.pack(*node))
    table.extend(text_blob)
    file_size=len(table)
    header=HEADER.pack(crc16(bytes(table[data_start:])),n,n,1,data_start,collision_count+1,file_size)
    table[:HEADER.size]=header
    Path(path).write_bytes(table)

if __name__=='__main__':
    import sys
    for file in sys.argv[1:]:
        rows=read_tbl(file)
        print(f'{file}: {len(rows)} strings; first={rows[0]!r}; cyrillic={sum(any("а"<=c.lower()<="я" for c in value) for _,value in rows)}')
