<template>
  <div class="articles-page">
    <div v-for="(article, index) in articles" :key="index" class="article">
      <div class="image"
        :style="'background-image: url(https://minecraft.net' + article.default_tile.image.imageURL + ')'">
      </div>
      <p>{{ article.default_tile.title }} <span>{{ article.default_tile.sub_header }}</span></p>
    </div>
  </div>
</template>
<!-- :style="'background-image: linear-gradient(0deg, rgba(0, 0, 0, 0.83), rgba(0, 0, 0, 0)), url(https://minecraft.net' + article.default_tile.image.imageURL + ')'" -->
<script setup lang="ts">
import { onMounted, ref } from 'vue'
import $ from 'jquery'
let articles: any = ref([])
// todo: use rust to fetch the data
// GET: https://www.minecraft.net/content/minecraft-net/_jcr_content.articles.grid?tileselection=auto&pageSize=2000&locale=zh-hans&lang=/content/minecraft-net/language-masters/zh-hans
onMounted(() => {
  console.log(1)
  $.get("/articles.json",
    function (data: any) {
      articles.value = data.article_grid
    },
  )
})
</script>

<style lang="less" scoped>
.articles-page {
  padding: 10px;
  flex-wrap: wrap;
  justify-content: center;
}

.article {
  width: 270px;
  height: 200px;
  margin: 10px;
  border-radius: var(--border-radius-large);
  display: inline-flex;
  align-items: flex-end;
  overflow: hidden;
  position: relative;
  cursor: pointer;
}

.article .image {
  width: 100%;
  height: 100%;
  background-position: center;
  background-size: cover;
  transition: transform 200ms ease;
}

.article:hover .image {
  transform: scale(1.1);
}

.article p {
  width: 100%;
  height: 100%;
  text-align: center;
  padding-top: 30px;
  color: #fff;
  font-family: 'minecraft_ten';
  font-size: 20px;
  opacity: 1;
  background: #0000008f;
  transition: all 200ms ease;
  position: absolute;
}

.article:hover p {
  opacity: 0.6;
}

.article span {
  display: block;
  width: 70%;
  margin: 0 auto;
  font-size: 12px;
  color: #aeaeae;
  font-family: 'minecraft';
}
</style>