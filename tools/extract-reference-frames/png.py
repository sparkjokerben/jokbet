import sys, zlib, struct
def readpng(p):
    d=open(p,'rb').read(); pos=8; idat=b''; w=h=0
    while pos<len(d):
        l=struct.unpack('>I',d[pos:pos+4])[0]; t=d[pos+4:pos+8]; c=d[pos+8:pos+8+l]; pos+=12+l
        if t==b'IHDR': w,h,bd,ct=struct.unpack('>IIBB',c[:10])
        elif t==b'IDAT': idat+=c
    raw=zlib.decompress(idat); bpp=4; stride=w*bpp; px=[]; prev=bytearray(stride); i=0
    for y in range(h):
        f=raw[i]; i+=1; line=bytearray(raw[i:i+stride]); i+=stride
        for x in range(stride):
            a=line[x-bpp] if x>=bpp else 0; b=prev[x]; c=prev[x-bpp] if x>=bpp else 0
            if f==1: line[x]=(line[x]+a)&255
            elif f==2: line[x]=(line[x]+b)&255
            elif f==3: line[x]=(line[x]+(a+b)//2)&255
            elif f==4:
                p_=a+b-c; pa=abs(p_-a); pb=abs(p_-b); pc=abs(p_-c)
                pr=a if pa<=pb and pa<=pc else (b if pb<=pc else c); line[x]=(line[x]+pr)&255
        px.append(bytes(line)); prev=line
    return w,h,px
