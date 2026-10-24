#!/usr/bin/env python3
"""Fetch exact primary native source snapshots, verifying size and SHA-256."""
import hashlib
import json
from pathlib import Path
import urllib.request

ROOT=Path(__file__).resolve().parents[1]
def digest(path):
    with path.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def main():
    folder=ROOT/'third_party/source-archives';folder.mkdir(parents=True,exist_ok=True)
    for item in json.loads((ROOT/'third_party/native-sources.json').read_text()):
        final=folder/item['file']
        if final.is_file() and final.stat().st_size==item['bytes'] and digest(final)==item['sha256']:
            print('Verified',final.name);continue
        temp=final.with_suffix(final.suffix+'.part')
        try:
            with urllib.request.urlopen(item['url'],timeout=60) as response,temp.open('wb') as output:
                size=0
                while chunk:=response.read(1024*1024):
                    size+=len(chunk)
                    if size>item['bytes']:raise ValueError('Source archive too large')
                    output.write(chunk)
            if size!=item['bytes'] or digest(temp)!=item['sha256']:raise ValueError('Source archive mismatch')
            temp.replace(final);print('Fetched and verified',final.name)
        finally:
            temp.unlink(missing_ok=True)
if __name__=='__main__':main()
