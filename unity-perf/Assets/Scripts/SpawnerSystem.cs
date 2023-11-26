using Unity.Entities;
using Unity.Burst;
using Unity.Mathematics;
using Unity.Collections;
using Unity.Transforms;

public partial struct SpawnerSystem : ISystem
{
    [BurstCompile]
    public void OnCreate(ref SystemState state)
    {
        state.RequireForUpdate<SpawnerComponent>();
        state.RequireForUpdate<FileSpawnerInfo>();
    }

    [BurstCompile]
    public void OnUpdate(ref SystemState state)
    {
        var spawner = SystemAPI.GetSingleton<SpawnerComponent>();
        var fileInfo = SystemAPI.GetSingleton<FileSpawnerInfo>();

        if (fileInfo.Exists)
        {
            spawner.count = fileInfo.count;
            spawner.maxSize = fileInfo.maxSize;
            spawner.robotSpeed = fileInfo.robotSpeed;
            SystemAPI.SetSingleton(spawner);
        }

        var instances = state.EntityManager.Instantiate(spawner.item, spawner.count, Allocator.Temp);
        state.EntityManager.AddComponent<RobotTarget>(instances);
        var random = Random.CreateFromIndex(1000);
        foreach (var entity in instances)
        {
            var transform = SystemAPI.GetComponentRW<LocalTransform>(entity);
            var pos = (random.NextFloat2() - new float2(0.5f, 0.5f)) * spawner.maxSize;
            transform.ValueRW.Position = new float3(pos.x, 2f, pos.y);
        }

        var robots = state.EntityManager.Instantiate(spawner.robot, spawner.count, Allocator.Temp);
        state.EntityManager.AddComponent<Robot>(robots);
        foreach (var entity in robots)
        {
            var transform = SystemAPI.GetComponentRW<LocalTransform>(entity);
            var pos = (random.NextFloat2() - new float2(0.5f, 0.5f)) * spawner.maxSize;
            transform.ValueRW.Position = new float3(pos.x, 2f, pos.y);
        }

        state.Enabled = false;
    }
}
