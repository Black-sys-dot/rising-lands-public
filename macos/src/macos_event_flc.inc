/* In-game FLC events (lane B contract motor/include/event-flc.md). The table is
 * one entry per distinct EVENEMENT value of EVENTS.INI, checked against
 * <payload>/events/NAME.FLC (the bundled or imported copy of the CD's Events).
 * The engine ABI exists from lane B's branch on; without it (MACOS_EVENT_FLC
 * undefined) the table is still built and testable, and the state is idle. */
#include <ctype.h>
static int event_flc_name_ok(const char*name){
 size_t n=strlen(name);if(n<1||n>15)return 0;
 for(size_t i=0;i<n;i++)if(!isalnum((unsigned char)name[i])&&name[i]!='_')return 0;return 1;
}
/* root: native-inputs (EVENTS.INI); events: folder of NAME.FLC. Sorted, distinct. */
int32_t macos_live_event_flc_table(const char*root,const char*events,MacOSEventFlcEntry*out,uint32_t capacity,uint32_t*count){
 if(!root||!events||!count)return RL_INVALID_ARGUMENT;
 uint32_t n=0;uint8_t*ini=file(root,"EVENTS.INI",&n);if(!ini)return RL_IO_ERROR;
 char names[256][16];uint32_t found=0;int section=0;
 for(uint32_t i=0;i<n;){
  uint32_t j=i;while(j<n&&ini[j]!='\n'&&ini[j]!='\r')j++;
  char line[256];uint32_t len=j-i<255?j-i:255;memcpy(line,ini+i,len);line[len]=0;
  i=j;while(i<n&&(ini[i]=='\n'||ini[i]=='\r'))i++;
  char*s=line;while(*s==' '||*s=='\t')s++;
  if(*s=='['){section=strncasecmp(s,"[EVENEMENT]",11)==0;continue;}
  char*eq=strchr(s,'=');if(!section||!eq)continue;
  char*v=eq+1;while(*v==' '||*v=='\t')v++;
  char*e=v+strlen(v);while(e>v&&(e[-1]==' '||e[-1]=='\t'))*--e=0;
  for(char*c=v;*c;c++)*c=(char)toupper((unsigned char)*c);
  if(!event_flc_name_ok(v)){free(ini);return RL_INVALID_ARGUMENT;}
  int dup=0;for(uint32_t k=0;k<found;k++)if(!strcmp(names[k],v))dup=1;
  if(!dup){if(found==256){free(ini);return RL_INVALID_ARGUMENT;}snprintf(names[found++],16,"%s",v);}
 }
 free(ini);
 qsort(names,found,16,(int(*)(const void*,const void*))strcmp);
 *count=found;if(!out)return found?RL_BUFFER_TOO_SMALL:0;if(capacity<found)return RL_BUFFER_TOO_SMALL;
 for(uint32_t k=0;k<found;k++){
  memset(&out[k],0,sizeof(out[k]));memcpy(out[k].name,names[k],strlen(names[k]));
  char path[4096];snprintf(path,sizeof(path),"%s/%s.FLC",events,names[k]);
  FILE*f=fopen(path,"rb");
  if(f){out[k].available=fread(out[k].header,1,128,f)==128;fclose(f);}
  if(!out[k].available)memset(out[k].header,0,128);
 }
 return 0;
}
static void event_flc_folder(MacOSLiveDriver*d,char*out,size_t size){snprintf(out,size,"%s/../events",d->root);}
#ifdef MACOS_EVENT_FLC
_Static_assert(sizeof(MacOSEventFlcEntry)==sizeof(RlEventFlcV1),"FLC table ABI size");
_Static_assert(sizeof(MacOSEventFlcState)==sizeof(RlEventFlcStateV1),"FLC state ABI size");
/* The engine keeps the declared table in its own world block (0x06300000). */
static int event_flc_present(MacOSLiveDriver*d,RlMundo world){
 uint32_t n=0;if(d->rl_projetar_tudo(world,NULL,0,&n)!=RL_BUFFER_TOO_SMALL)return 0;
 uint8_t*data=malloc(n);if(!data)return 0;int found=0;
 if(!d->rl_projetar_tudo(world,data,n,&n)&&n>=4){
  uint32_t off=4;for(uint32_t i=0;i<le(data);i++){
   if((uint64_t)off+8>n)break;uint32_t base=le(data+off),length=le(data+off+4);off+=8;
   if((uint64_t)off+length>n)break;if(base==0x06300000)found=1;off+=length;
  }
 }
 free(data);return found;
}
/* Every new or loaded world, before its first frame. A world restored from a
 * save (or resumed from the pause menu) already carries its table and is left
 * as it is, without reading EVENTS.INI; any other refusal is reported. */
static int event_flc_declare(MacOSLiveDriver*d,RlMundo world){
 if(!d->rl_eventos_flc_v1||!world)return 0;
 if(event_flc_present(d,world))return 0;
 MacOSEventFlcEntry table[256];uint32_t count=0;char events[4096];event_flc_folder(d,events,sizeof(events));
 int c=macos_live_event_flc_table(d->root,events,table,256,&count);
 if(c){snprintf(d->save_error,sizeof(d->save_error),"Tabela de eventos FLC indisponível (%d)",c);return c;}
 if(!count)return 0;
 c=d->rl_eventos_flc_v1(world,(const RlEventFlcV1*)table,count);
 if(c){
  char detail[320]={0};uint32_t n=0;d->rl_erro_mensagem(NULL,0,&n);if(n&&n<=sizeof(detail))d->rl_erro_mensagem((uint8_t*)detail,n,&n);
  if(strstr(detail,"already declared"))return 0;
  snprintf(d->save_error,sizeof(d->save_error),"Tabela de eventos FLC recusada: %s",detail);
 }
 return c;
}
int32_t macos_live_event_flc_state(MacOSLiveDriver*d,MacOSEventFlcState*out){
 if(!d||!out)return RL_INVALID_ARGUMENT;memset(out,0,sizeof(*out));
 if(!d->rl_evento_flc_estado_v1||!d->world||d->shell_mode)return 0;
 return d->rl_evento_flc_estado_v1(d->world,(RlEventFlcStateV1*)out);
}
int32_t macos_live_event_flc_finish(MacOSLiveDriver*d){
 if(!d||!d->rl_evento_flc_terminar_v1||!d->world)return RL_INVALID_ARGUMENT;
 return d->rl_evento_flc_terminar_v1(d->world);
}
uint32_t macos_live_event_flc_supported(void){return 1;}
#else
static int event_flc_declare(MacOSLiveDriver*d,RlMundo world){(void)d;(void)world;return 0;}
int32_t macos_live_event_flc_state(MacOSLiveDriver*d,MacOSEventFlcState*out){if(!d||!out)return RL_INVALID_ARGUMENT;memset(out,0,sizeof(*out));return 0;}
int32_t macos_live_event_flc_finish(MacOSLiveDriver*d){(void)d;return RL_UNSUPPORTED_PATH;}
uint32_t macos_live_event_flc_supported(void){return 0;}
#endif
/* Name of the table entry `index` (sorted order, as declared). */
int32_t macos_live_event_flc_name(MacOSLiveDriver*d,uint32_t index,char*out,uint32_t capacity){
 if(!d||!out||capacity<16)return RL_INVALID_ARGUMENT;
 MacOSEventFlcEntry table[256];uint32_t count=0;char events[4096];event_flc_folder(d,events,sizeof(events));
 int c=macos_live_event_flc_table(d->root,events,table,256,&count);if(c)return c;
 if(index>=count)return RL_INVALID_ARGUMENT;
 memcpy(out,table[index].name,16);out[15]=0;return 0;
}
