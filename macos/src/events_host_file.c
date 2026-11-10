/* Host transport only; no original game filename is interpreted as a host path. */
#include "events_host_file.h"
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
int32_t rl_events_file_read_v1(int parent,const uint8_t*logical,uint32_t length,RlOwnedEventsV1*out){
 if(parent<0||!logical||!out||length<4||length>244||memchr(logical,0,length))return RL_INVALID_ARGUMENT;
 struct stat dir,before,after;if(fstat(parent,&dir)||!S_ISDIR(dir.st_mode))return RL_INVALID_ARGUMENT;
 int fd=openat(parent,"EVENTS.INI",O_RDONLY|O_NOFOLLOW|O_CLOEXEC|O_NONBLOCK);
 if(fd<0)return RL_IO_ERROR;
 int c=RL_IO_ERROR;uint8_t*file=NULL,*directory=NULL;
 if(fstat(fd,&before)||!S_ISREG(before.st_mode)||before.st_size<=0||before.st_size>1024*1024)goto done;
 file=malloc((size_t)before.st_size);directory=malloc(length);if(!file||!directory)goto done;
 size_t at=0;while(at<(size_t)before.st_size){ssize_t n=read(fd,file+at,(size_t)before.st_size-at);if(n<0&&errno==EINTR)continue;if(n<=0)goto done;at+=(size_t)n;}
 uint8_t extra;if(read(fd,&extra,1)!=0||fstat(fd,&after)||before.st_dev!=after.st_dev||before.st_ino!=after.st_ino||before.st_size!=after.st_size||before.st_mtime!=after.st_mtime||before.st_ctime!=after.st_ctime||memchr(file,0,at))goto done;
 memcpy(directory,logical,length);
 *out=(RlOwnedEventsV1){{sizeof(RlEventsHostV1),1,directory,length,file,(uint32_t)at},directory,file};file=NULL;directory=NULL;c=RL_OK;
 done:free(file);free(directory);close(fd);return c;
}
void rl_events_file_free_v1(RlOwnedEventsV1*owned){if(!owned)return;free(owned->directory);free(owned->events);memset(owned,0,sizeof(*owned));}
