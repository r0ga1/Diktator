"""Regenerate only the needed ABI structs from the pinned sherpa-onnx 1.13.8 header."""
import pathlib,re,sys
header=pathlib.Path(sys.argv[1]).read_text();header=re.sub(r'/\*.*?\*/','',header,flags=re.S)
structs=dict((name,body) for name,body in re.findall(r'typedef struct (SherpaOnnx\w+)\s*\{(.*?)\}\s*\w+\s*;',header,re.S))
needed=set()
def visit(name):
 if name in needed:return
 needed.add(name)
 for dep in re.findall(r'SherpaOnnx\w+',structs[name]):visit(dep)
visit('SherpaOnnxOfflineRecognizerConfig');visit('SherpaOnnxOfflineRecognizerResult')
lines=['// ABI structs derived from sherpa-onnx 1.13.8 c-api.h. Apache-2.0, Xiaomi Corporation.','#![allow(non_camel_case_types)]','use std::ffi::c_char;']
for name,body in structs.items():
 if name not in needed:continue
 lines+=['#[repr(C)]','#[derive(Clone,Copy)]',f'pub struct {name} {{']
 for field in body.split(';'):
  field=field.strip()
  if not field:continue
  typ,ident=re.match(r'(.+?)(\w+)\s*$',field,re.S).groups();typ=' '.join(typ.split())
  if '*' in typ:
   if typ=='const char *const *':t='*const *const c_char'
   elif typ=='const char *':t='*const c_char'
   elif typ=='const float *':t='*const f32'
   elif typ=='float *':t='*mut f32'
   else:raise ValueError(typ)
  else:t={'int32_t':'i32','float':'f32'}.get(typ,typ)
  lines.append(f' pub {ident}: {t},')
 lines+=['}',f'impl Default for {name} {{ fn default()->Self {{ unsafe {{ std::mem::zeroed() }} }} }}']
pathlib.Path(__file__).resolve().parents[1].joinpath('src/asr_ffi.rs').write_text('\n'.join(lines)+'\n')
print('Generated',len(needed),'ABI structs')
