<script setup>
import { ref ,onMounted } from "vue";
import { invoke } from "@tauri-apps/api";
import { readBinaryFile } from "@tauri-apps/api/fs";
import { listen } from '@tauri-apps/api/event'
defineProps({
  msg: String,
  src: String,
});
const text = ref(0);
const img = ref("");
const ocr = function () {
  invoke("img").then((response) => {
    console.log(response);
    readBinaryFile(response).then((data) => {
      let binary_data_arr = new Uint8Array(data);
      let p = new Blob([binary_data_arr], { type: "image/png" });
      img.value = URL.createObjectURL(p);
    });
    invoke("ocr").then((r) => {
      text.value = r;
    });
  });
};
onMounted(() => {
  listen('ocr', event => {
    console.log(event)
    ocr()
  })
  invoke("shortcut").then(r =>{

  })
});
</script>

<template @dbclick="ocr">
  <div v-if="img">
    <img alt="Vue logo" :src="img" title="剪贴板中的图片用于OCR识别"/>
    <h3>OCR成功，结果已复制可以直接粘贴文本</h3>
    <textarea v-model="text"></textarea>
  </div>

  <button type="button" @click="ocr">识别粘贴板中内容Alt+C</button>
</template>

<style scoped>
h3{
  color:green;
}
button {
  height: 40px;
  margin-top: 50px;
  color: blue;
  background: white;
  border: 1px solid #1f6de3;
  font-size:16px;
  border-radius: 10px 10px 10px 10px;
  box-shadow: #1f6de3 3px 3px 3px 1px;
}
a {
  color: #42b983;
}
img {
  border: 1px solid green;
  max-width: 80%;
  max-height: 300px;
  box-shadow: #5188da 3px 3px 3px;
}
textarea {
  border: 1px solid #42b983;
  min-height: 120px;
  min-width:90%;
  margin: 20px;
  padding: 5px;
  box-shadow: #5188da 3px 3px 3px;
}
</style>
