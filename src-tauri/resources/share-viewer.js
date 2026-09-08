const image=document.getElementById('live'),status=document.getElementById('liveStatus');
async function update(){
 if(document.hidden){setTimeout(update,1000);return;}
 try{const response=await fetch('live.jpg',{cache:'no-store',signal:AbortSignal.timeout(5000)});if(!response.ok)throw new Error('off');const blob=await response.blob();const url=URL.createObjectURL(blob);const old=image.src;image.src=url;image.style.display='block';if(old.startsWith('blob:'))URL.revokeObjectURL(old);status.textContent='Live · 실시간 (약 2 fps · no audio)';}
 catch{if(image.src.startsWith('blob:'))URL.revokeObjectURL(image.src);image.style.display='none';image.removeAttribute('src');status.textContent='Live view is off or unavailable · 화면 공유 꺼짐 또는 연결 대기';}
 setTimeout(update,500);
}
update();
